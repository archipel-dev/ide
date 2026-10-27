# 04. The Java error catalog

Status: specification, clean-room. Written from the IntelliJ IDEA Community sources in our own words.
Message strings are quoted from the message bundles; they are short user-visible facts. Source paths are
cited as `ic: <path>` relative to the Community repository root.

Primary sources: `ic: java/codeserver/highlighting/` (the checker set that produces compile-time errors:
class, method, expression, statement, control-flow, generics, switch, pattern, annotation, record, module,
import, literal and modifier checks, plus the table of error kinds and their message formatting),
`ic: java/codeserver/highlighting/resources/messages/JavaCompilationErrorBundle.properties`,
`ic: java/java-psi-impl/resources/messages/JavaErrorBundle.properties` (older parse and
warning texts), `ic: java/java-analysis-impl/.../DefaultJavaErrorFixProvider.java` and
`ic: java/java-impl-inspections/.../AdditionalJavaErrorFixProvider.java` (which quick fixes hang on which
error), `ic: java/java-analysis-impl/.../HighlightFixUtil.java`.

How to read the tables. *Message* is the bundle text with the quote rule applied (the bundle writes a
single quote as two; here it is one) and `{n}` placeholders described in 4.1. *Range* is the text that is
underlined. *Pri* is `MUST`, `SHOULD` or `LATER`. *Fixes* are the quick fixes in registration order (the
order they appear under Alt+Enter, subject to priority bands: a fix with top priority appears first).
All entries are severity **ERROR** unless the table says otherwise.

---

## 4.1 Common rules

### 4.1.1 The report model

An error has: a kind (identified here by its message key), the PSI element it is reported on (the
*anchor*), an optional narrowing *range* inside the anchor, a highlight style, a description (the
message), an optional HTML tooltip, an optional navigation shift (where the caret goes when the user
jumps to the error with F2), and a context object used by the fixes.

- **Default range** is the anchor's whole text range. Entries below state the exceptions.
- **Header ranges.** "Method header" = from the first token after the last annotation of the modifier
  list through the end of the throws list (or to the end of the method when it has no body).
  "Class header" = from the first token after annotations of the modifier list through the end of the
  implements list (or the name when there is none); for an anonymous class, the base class reference.
  "Field header" = from the first token after annotations through the field name. These keep the red
  underline off annotations and bodies.
- **Styles**: `ERROR` (red wavy underline), `WRONG_REF` (the "unresolved reference" text attribute: red
  wavy underline on identifiers; also drives the auto-import hint), `UNHANDLED_EXCEPTION` (same red,
  distinct attribute key so users can restyle it), `FILE_LEVEL` (banner on the whole file instead of a
  range, for implicit-class problems), `PENDING_REF` (soft grey-blue underline, see 4.1.3).

### 4.1.2 Message placeholders

- Types in messages are printed in **internal canonical text**: fully qualified, with type arguments,
  e.g. `java.lang.String`, `java.util.List<java.lang.Integer>`; primitives as keywords; `null` for a
  missing type. Exceptions: the "cannot resolve method" family prints argument types with **short
  presentable names** (`String`, `List<Integer>`), unknown types as `?`; and the unused/deprecation
  messages print symbol names (class simple name, method name with parentheses).
- Methods format as `name(paramType, paramType)` using simple type names; classes as their name (the
  qualified name when a clash needs it); `{2}` argument lists in wrong-argument errors as
  `(java.lang.String, int)`.
- Singular/plural forms use the choice patterns embedded in the messages.

### 4.1.3 Precedence and suppression

- **One error per element chain.** Each visit performs its checks in a fixed order and stops as soon as
  one reports an error ("has error results"). A later check on the same element does not report. The
  order matters: syntax errors suppress everything on the same node; resolve errors precede type checks.
- **Syntax errors** come from the parser: an error element is reported as `Syntax error` with the parser's
  description (4.9). An element that contains parse errors is skipped by most semantic checks (for
  example an argument list with parse errors suppresses wrong-argument errors).
- **Incomplete project model.** While the project model is still being loaded (dependencies unresolved),
  an unresolved reference that could become resolvable is reported as `Not resolved until the project is
  fully loaded` with `PENDING_REF` style instead of an error; type-conversion checks are relaxed when an
  operand's type has unresolved parts; calls whose argument types are unresolved are not reported as
  mismatches. (`ic: java/java-psi-impl/.../IncompleteModelUtil.java`.)
- **Language level.** A construct above the file's language level is reported as `{0} are not supported at
  language level '{1}'` (for example `Lambda expressions are not supported at language level '7'`) with the
  fixes to raise the module/project language level.

- **JAVA-ERR-001** `MUST` Report model of 4.1.1 (anchor, range, style, tooltip, navigation).
- **JAVA-ERR-002** `MUST` Check chains stop at the first error per element visit.
- **JAVA-ERR-003** `MUST` Pending-reference behavior while the project model is incomplete.
- **JAVA-ERR-004** `MUST` Header ranges that exclude annotations (4.1.1).
- **JAVA-ERR-005** `SHOULD` Language-level gate messages and the raise-language-level fixes.
- **JAVA-ERR-006** `SHOULD` Tooltips for incompatible types and mismatched calls (HTML tables
  `Required type:` / `Provided:`).

---

## 4.2 Resolve and reference errors

| ID | Pri | Message | Range | Trigger | Fixes |
|---|---|---|---|---|---|
| JAVA-ERR-007 | MUST | `Cannot resolve method '{0}'` where `{0}` = `name(shortArgTypes)` | method name, WRONG_REF | A method call whose method name yields **zero candidates** (no method of that name visible, accessibility aside) | see 5.1.2: Import static method, Qualify static call, Add qualifier, Create method, create-from-usage variants, static-qualifier variants; also the "wrap/cast/permute" families only apply when candidates exist |
| JAVA-ERR-008 | MUST | `Cannot resolve method '{0}'` (same text) | the **argument list** including parentheses | Candidates with this name exist (overloads) and **none is applicable**; all arguments have known types; no two are applicable | for each candidate: cast argument, change signature (add/remove parameter), permute arguments, wrap with adapter (`Optional.of`, `List.of`, ...), surround with array, change parameter class, add explicit type arguments, move parenthesis; static-context fix if single candidate in wrong context |
| JAVA-ERR-009 | MUST | `'{0}' in '{1}' cannot be applied to '{2}'` (`{0}` method signature, `{1}` containing class, `{2}` `(argTypes)` fully qualified). When the argument count differs the message is instead `Expected {n} arguments but found {m}` (forms: `Expected no arguments but found 1`, `Expected 1 argument but found 2`) | whole argument list when every argument is wrong or the count differs; otherwise **each mismatched argument** separately; tooltip is an HTML table | Exactly one candidate, accessible, but arguments not applicable | change method signature, cast argument, wrap argument, permute, remove redundant arguments, change parameter type, qualify, add type arguments |
| JAVA-ERR-010 | MUST | `Ambiguous method call: both '{0}' and '{1}' match` | argument list | Two or more candidates are applicable and none is most specific (JLS 15.12.2.5) | specify types/cast (cast argument), change signature fixes |
| JAVA-ERR-011 | MUST | `Cannot resolve method '{0}' in '{1}'` | method name, WRONG_REF | Qualifier is a type/class and no candidate is accessible | same as 007 family, keyword-typo fix, access fixes |
| JAVA-ERR-012 | MUST | `Cannot resolve symbol '{0}'` | reference name, WRONG_REF | Unresolved simple or qualified name of a class, variable, package or field (not a method name) | In order: add library/module dependency to the module when a class of that name exists in a library or module not on the classpath; move class to module; (for expressions) create local variable, create field (several targets), create parameter, rename reference, add qualifier cast, bring variable into scope; remove `new` when the target is a static call; keyword-typo fix; **last and lazily**: Import static constant, Qualify static constant, **Import class** (top priority band, so shown first) - see 5.5 |
| JAVA-ERR-013 | MUST | `Reference to '{0}' is ambiguous, both '{1}' and '{2}' match` (the two descriptions sorted alphabetically) | reference name, WRONG_REF | A name resolves to two members that are both accessible and not hiding each other (e.g. a field inherited from two interfaces; a class imported from two on-demand packages) | qualify fixes |
| JAVA-ERR-014 | MUST | `'{0}' has private access in '{1}'` | name | Resolved member is private and the place is outside the top-level class (JLS 6.6.1) | change visibility, qualify |
| JAVA-ERR-015 | MUST | `'{0}' has protected access in '{1}'` | name | Protected member accessed from another package outside a subclass context (JLS 6.6.2) | make public/protected, qualify |
| JAVA-ERR-016 | MUST | `'{0}' is not public in '{1}'. Cannot be accessed from outside package` | name | Package-private member from another package | make public |
| JAVA-ERR-017 | SHOULD | `Cannot access '{0}' in '{1}'` | name | Other access failure (module encapsulation, inaccessible type in signature) | |
| JAVA-ERR-018 | MUST | `Non-static {0} '{1}' cannot be referenced from a static context` (`{0}` = `field`, `method`, `variable`, ...) | name, WRONG_REF | Instance member referenced from static method/initializer/static nested context without an instance | make member static, make context non-static, create instance; for calls the method-call fixes too |
| JAVA-ERR-019 | MUST | `Cannot access fields on '{0}' type` / `Cannot call methods on '{0}' type` | name, WRONG_REF | Member access on a primitive-typed qualifier (`int x; x.foo()`) | rename reference |
| JAVA-ERR-020 | SHOULD | `Method call expected` | call name | A class name is used where a method call was meant, or a call of a non-method symbol | insert `new`, static import/qualify, create method |
| JAVA-ERR-021 | SHOULD | `Cannot resolve constructor '{0}'` (`{0}` = `Class(shortArgTypes)`) | **argument list** of the `new` / `this(..)` / `super(..)` | No applicable constructor | create constructor, change signature fixes, cast/wrap arguments |
| JAVA-ERR-022 | SHOULD | `There is no no-arg constructor available in '{0}'` | header of the subclass or constructor that implicitly calls `super()` | Implicit super constructor call but superclass has constructors and none is no-arg | add no-arg constructor to superclass, create constructor matching super, add `super(..)` |
| JAVA-ERR-023 | SHOULD | `Unknown class: '{0}'` | type element | Type element naming an unresolvable class in a context where resolution failure is reported via the type | same as 012 |
| JAVA-ERR-024 | SHOULD | `Class '{0}' is in the default package` | reference name, WRONG_REF | Class in the unnamed package referenced from a named package or from an import | move class |
| JAVA-ERR-025 | SHOULD | `Cannot select from a type parameter` | whole reference | `T.foo()` where `T` is a type variable | |
| JAVA-ERR-026 | SHOULD | `Qualifier must be an expression` | whole qualifier | A method call whose qualifier resolves to a non-expression symbol (method, package) | |
| JAVA-ERR-027 | SHOULD | `Package not found: {0}` / `Cannot resolve package {0}` | reference | Unknown package in a qualified name | |
| JAVA-ERR-028 | SHOULD | `Cannot reference '{0}' before superclass constructor is called` / `Cannot call '{0}' before superclass constructor is called` | member name | Using instance member in `this(...)`/`super(...)` arguments | |
| JAVA-ERR-029 | SHOULD | `Cannot read value of field '{0}' before the field's definition` / `Cannot read value of field '{0}' from inside the field's definition` | name | Illegal forward/self reference of a field in an initializer (JLS 8.3.3) | |
| JAVA-ERR-030 | LATER | `'{0}' cannot be referenced from a static context` | name | Outer type parameter / non-static class in static context | |
| JAVA-ERR-031 | LATER | `Static method may only be called on its containing interface` | name | `instance.staticInterfaceMethod()` or via subinterface (JLS 15.12.3) | |
| JAVA-ERR-032 | LATER | `Local class '{0}' cannot be referenced from another switch branch` | name | | |

Notes for 007/008/009 selection. After resolving the method name:
(a) no candidates: 007; (b) candidates exist, the best one is accessible and statics context correct but not applicable: if
there is exactly one candidate overall, 009; with several, 008; (c) candidate not accessible: access
errors 014-017 on the name; (d) candidate in wrong static context: 018; (e) two applicable: 010.
The error is withheld when any argument has an unknown type (the argument error is reported on the
argument itself) or while the model is incomplete.

---

## 4.3 Type errors

| ID | Pri | Message | Range | Trigger | Fixes |
|---|---|---|---|---|---|
| JAVA-ERR-033 | MUST | `Incompatible types. Found: '{1}', required: '{0}'` (`{0}` target type, `{1}` source type, both canonical; the tooltip is a table `Required type: ... Provided: ...` plus a grey reason line such as `reason: no instance(s) of type variable(s) T exist so that List<T> conforms to String`) | the offending expression (initializer, assignment right side, return value, argument in some cases); for the right side of an assignment `a = b` **the whole assignment expression** | The expression type is not assignable to the target type in the assignment/initializer/return/condition context (JLS 5.2, 5.1; `03-types.md`) | change variable type, change method return type, cast, wrap in `Optional`/array/list, convert `=` to `==`, replace primitive with boxed, change `new` operator type, adapt expression type (parse/`String.valueOf`/`toString`), add type arguments to conditional, make variable `var`... |
| JAVA-ERR-034 | MUST | `Inconvertible types; cannot cast '{0}' to '{1}'` | whole cast or `instanceof` expression | Cast/instanceof between types not related by a casting conversion (JLS 5.5, 15.20.2) | replace primitive with boxed (for instanceof of a primitive) |
| JAVA-ERR-035 | MUST | `Operator '{0}' cannot be applied to '{1}', '{2}'` (compound assignment prints the base operator without `=`) | the operator token for compound assignments; the **whole binary expression** otherwise | Operand types violate the operator rules (JLS 15.17-15.24) | none specific (type-change fixes of operands) |
| JAVA-ERR-036 | MUST | `Operator '{0}' cannot be applied to '{1}'` | whole unary expression | unary operator misuse (`!` on int) | negation/boolean conversion fixes |
| JAVA-ERR-037 | MUST | `Foreach not applicable to type '{0}'` | the expression after the colon | Not an array or `Iterable` (JLS 14.14.2) | implement `Iterable`-style fixes |
| JAVA-ERR-038 | SHOULD | `Array type expected; found: '{0}'` | the indexed expression | `x[0]` where x is not an array | replace with list access when `List` |
| JAVA-ERR-039 | MUST | `Reference to variable expected on left-hand side of assignment` / `Reference to variable expected as target of unary operation` | the left side / operand | `5 = x`, `foo()++` | |
| JAVA-ERR-040 | SHOULD | `Expression expected` | the element | A non-value (class name, void call used as value, package) used as an expression | |
| JAVA-ERR-041 | SHOULD | `'void' type is not allowed here` / `Illegal type: 'void'` | the void expression/type | void call as operand, `void` array/variable type | |
| JAVA-ERR-042 | SHOULD | `Type argument cannot be of a primitive type` | the type argument | `List<int>` (JLS 4.5.1) | replace with boxed type |
| JAVA-ERR-043 | SHOULD | `Wrong number of type arguments: {0}; required: {1}` | the type argument list (navigation to first extra) | `Map<String>` | |
| JAVA-ERR-044 | SHOULD | `Type '{0}' does not have type parameters` / `Method '{0}' does not have type parameters` | the argument list | `String<Integer>` | delete type arguments |
| JAVA-ERR-045 | SHOULD | `Type parameter '{0}' is not within its bound; should extend '{1}'` (variant `should implement`) | the type argument | Bound violation (JLS 4.5) | add bound |
| JAVA-ERR-046 | SHOULD | `Inferred type '{2}' for type parameter '{0}' is not within its bound; should extend '{1}'` | method call name | Inference produced a type outside the bound | |
| JAVA-ERR-047 | SHOULD | `{0}` (the inference engine's explanation, e.g. `inference variable T has incompatible bounds: equality constraints: String lower bounds: Integer`) | method name (or for the whole call when no name) | The call is applicable by arity but inference fails | change types; add explicit type arguments |
| JAVA-ERR-048 | SHOULD | `Type arguments given on a raw type` / `Type arguments given on a raw method` | argument list | | |
| JAVA-ERR-049 | SHOULD | `Generic array creation not allowed` | the `new` expression | `new List<String>[3]` (JLS 15.10.1) | |
| JAVA-ERR-050 | SHOULD | `Diamond operator is not applicable to non-parameterized types` / `Diamond operator is not allowed here` | the type argument list | `new String<>()`; diamond with anonymous inner non-private members below Java 9 | delete diamond |
| JAVA-ERR-051 | SHOULD | `Wildcard type '{0}' cannot be instantiated directly` / `Wildcards may only be used as reference parameters` | the type | | |
| JAVA-ERR-052 | SHOULD | `Class cannot inherit from its type parameter` / `Generic class may not extend 'java.lang.Throwable'` | the extends reference | JLS 8.1.2, 8.1.4 | |
| JAVA-ERR-053 | SHOULD | `'{0}' cannot be inherited with different type arguments: '{1}' and '{2}'` / `'{0}' cannot be inherited as a raw type and with generic type arguments '{1}'` | class header | JLS 8.1.5 | |
| JAVA-ERR-054 | LATER | `Type parameter {0} has incompatible upper bounds: {1}` / `Duplicate type parameter: '{0}'` | | | |
| JAVA-ERR-055 | SHOULD | `Cannot infer type: 'var' on variable without an initializer` (and variants: `lambda expression requires an explicit target type`, `method reference requires...`, `variable initializer is 'null'`, `variable initializer is 'void'`, `it is used in its own variable initializer`, `'var' is not allowed in a compound declaration`, `'var' is not allowed as an element type of an array`) | the `var` type element | Local-variable type inference misuse (JLS 14.4.1) | specify explicit type (`Specify type explicitly`) |
| JAVA-ERR-056 | SHOULD | `Type parameter '{0}' cannot be instantiated directly` | | `new T()` | |
| JAVA-ERR-057 | SHOULD | `Cannot catch type parameters` / `Class or array expected` / `Illegal generic type for instanceof` / `'{0}' cannot be safely cast to '{1}'` | type | catch/instanceof with generic misuse | |

Warnings related to types are not errors and are listed in `07-inspections.md`: unchecked assignment/cast/call
(`Unchecked assignment: '{0}' to '{1}'`, `Unchecked cast: '{0}' to '{1}'`, `Unchecked call to '{0}' as a member of raw
type '{1}'`, `Unchecked method '{0}' invocation`), raw type use, redundant cast.

---

## 4.4 Flow: return, reachability, initialization, final

All flow errors are computed from the control-flow graph of a method/lambda/initializer body per JLS 14.22
(reachability) and 16 (definite assignment). Constant conditions: for reachability, `if` conditions are
**not** constant-folded (JLS 14.22: `if (false) {..}` is reachable); `while (true)` and `for(;;)` are; loop
conditions folded per JLS.

| ID | Pri | Message | Range | Trigger | Fixes |
|---|---|---|---|---|---|
| JAVA-ERR-058 | MUST | `Missing return statement` | the closing `}` of the body (the last child when no brace) | A non-void method or lambda body block can complete normally (JLS 8.4.7, 15.27.3) | `Add 'return' statement`; `Make '{method}' return 'void'` |
| JAVA-ERR-059 | MUST | `Missing return value` | the `return` statement | `return;` in a non-void method | make method return void; change return to the expected type |
| JAVA-ERR-060 | MUST | `Cannot return a value from a method with void result type` | the `return` statement | `return x;` in void method / lambda void | make method return the expression type (and variant to the determined return type) |
| JAVA-ERR-061 | SHOULD | `Cannot return a value from a constructor` | the return statement | | |
| JAVA-ERR-062 | MUST | `Unreachable statement` | for `if`, `switch`, loops, `throw`, `return`, `yield`, `try`, `synchronized`, `assert` and labeled statements: the **leading keyword** (or label); for other statements the whole statement | A statement that cannot be reached by any path (JLS 14.22): after `return`, `break`, `continue`, `throw`, infinite loop | `Delete unreachable statement` |
| JAVA-ERR-063 | SHOULD | `Loop condition is always false making the loop body unreachable` | the loop body | `while (false) {..}`, `for (;false;)` (JLS 14.22) | |
| JAVA-ERR-064 | MUST | `Variable '{0}' might not have been initialized` | the variable reference (read) | A local or blank final read before definite assignment (JLS 16) | `Initialize variable '{0}'` (add initializer), for locals assigned in a `switch`: `Add 'default' branch to the 'switch' statement which initializes '{0}'`, for finals: make not final |
| JAVA-ERR-065 | MUST | `Variable '{0}' might already have been assigned to` | the assignment target | A final local/blank final assigned when possibly already assigned (JLS 16, 4.12.4) | defer final assignment / make not final |
| JAVA-ERR-066 | SHOULD | `Variable '{0}' might be assigned in a loop` | the assignment | final assigned inside loop | |
| JAVA-ERR-067 | MUST | `Field '{0}' might not have been initialized` | the field **header** (declaration without annotations) for a blank final field with no assignment in every constructor/initializer path | JLS 8.3.1.2, 16.8 | `Create constructor parameter for field`, `Initialize final field in constructor`, `Add initializer`, `Make '{field}' not final` |
| JAVA-ERR-068 | MUST | `Cannot assign a value to final variable '{0}'` | the assigned reference | Assignment to a final field/local/parameter (initialized or not blank final) (JLS 4.12.4, 15.26) | `Make '{0}' not final`; for inner-class captured variables variable-access fix; qualify |
| JAVA-ERR-069 | SHOULD | `Cannot assign final field '{0}' before|after chained constructor call` ; `Final field '{0}' is already initialized in another field initializer` / `in a class initializer` | | | |
| JAVA-ERR-070 | MUST | `Variable '{0}' is accessed from within inner class, needs to be final or effectively final` (pre-Java 8: `needs to be declared final`) | the captured variable reference | JLS 8.1.3: capturing a non-effectively-final local from an inner/anonymous class | `Make '{0}' effectively final` (introduce copy), variable-access-from-inner-class fix |
| JAVA-ERR-071 | MUST | `Variable used in lambda expression should be final or effectively final` | the reference | JLS 15.27.2 | same fixes |
| JAVA-ERR-072 | SHOULD | `Variable used as a try-with-resources resource should be final or effectively final` / `Variable used in guard expression should be final or effectively final` | | | |
| JAVA-ERR-073 | MUST | `'break' outside of switch or loop` / `'continue' statement outside of loop` | the statement | JLS 14.15, 14.16 | |
| JAVA-ERR-074 | SHOULD | `Undefined label: '{0}'` / `'continue' target must be a loop label: '{0}'` / `Label '{0}' already in use` / `Label without statement` | label | | |
| JAVA-ERR-075 | SHOULD | `Initializer must be able to complete normally` | the initializer | `static { throw ... }` that cannot complete (JLS 8.6, 8.7) | |
| JAVA-ERR-076 | SHOULD | `Break out of switch expression is not allowed` / `Continue out of switch expression is not allowed` / `Return outside of enclosing switch expression` | statement | JLS 15.28.1 | |
| JAVA-ERR-077 | LATER | `'yield' outside of a switch expression`, `Expression type should not be 'void'` | | | |

---

## 4.5 Exceptions

Checked-exception analysis per JLS 11.2: the set of checked exception classes a statement can throw
(method/constructor `throws` of the resolved target, `throw` statements with the static type or the
effectively-final rethrow analysis, implicit `close()` of resources, implicit super constructor calls and
field-initializer exceptions) versus the handlers (catch sections of enclosing try, `throws` of the
enclosing method, lambda functional-interface `throws`, initializers).

| ID | Pri | Message | Range | Trigger | Fixes |
|---|---|---|---|---|---|
| JAVA-ERR-078 | MUST | `Unhandled exception: {0}` (plural `Unhandled exceptions: {0}`; the list holds the canonical class names joined with a comma and one space, e.g. `java.io.IOException, java.sql.SQLException`) | for a method call: the **method name**; for `new X()` the class reference; for method references the name; for other elements the whole element; style UNHANDLED_EXCEPTION. Exceptions whose class cannot be resolved are dropped from the list; none left = no error | A checked exception can be thrown and is neither caught nor declared | In order: `Add exception to method signature` (`Add exceptions to method signature` for several; with a choice popup to change the whole hierarchy or only this method when the method overrides), `Surround with try/catch`, `Add 'catch' clause(s)` (when inside a try), `Surround with try/with-resources` when resource-capable, for constructors `Create constructor matching super`; throwing from a lambda: change the functional interface where possible |
| JAVA-ERR-079 | MUST | `Unhandled exception from auto-closeable resource: {0}` (plural analogous) | the resource variable/expression | `close()` of an `AutoCloseable` declares a checked exception (JLS 14.20.3) | same family |
| JAVA-ERR-080 | MUST | `Exception '{0}' has already been caught` | the type in the later catch | A catch type is a subclass of (or equal to) a type caught earlier in the same try (JLS 14.21) | `Delete catch for '{0}'`; for multi-catch alternative delete the alternative |
| JAVA-ERR-081 | MUST | `Exception '{0}' is never thrown in the corresponding try block` | the catch parameter | A checked exception class (other than `Exception`/`Throwable` and their superclasses of runtime exceptions) is caught but the try block cannot throw it (JLS 11.2.3) | `Delete catch for '{0}'` |
| JAVA-ERR-082 | SHOULD | `Types in multi-catch must be disjoint: '{0}' is a subclass of '{1}'` | the subclass alternative | JLS 14.20 | delete alternative |
| JAVA-ERR-083 | SHOULD | `Exception '{0}' is never thrown in the method` | the throws entry | (inspection-grade, in `JavaErrorBundle`) | |
| JAVA-ERR-084 | SHOULD | `Unhandled exception` for default constructors: the same message at the class header when an implicit `super()` throws | class header | | add constructor with throws |

---

## 4.6 Declarations: duplicates, abstractness, overriding, modifiers

| ID | Pri | Message | Range | Trigger | Fixes |
|---|---|---|---|---|---|
| JAVA-ERR-085 | MUST | `Variable '{0}' is already defined in the scope` | the new variable's name identifier; tooltip links to the earlier declaration | Same-scope or shadowing-of-local/parameter redeclaration (JLS 6.4) | `Navigate to already declared variable`, `Reuse variable declaration` (for local declarations: turn into assignment) |
| JAVA-ERR-086 | MUST | `'{0}' is already defined in '{1}'` (method signature, containing class) | method header | Two methods with the same signature after erasure-aware comparison in one class (JLS 8.4.2) | `Show duplicate elements` (when several, excluding synthetic methods) |
| JAVA-ERR-087 | MUST | `Duplicate class: '{0}'` | class name identifier (file-level style for implicit classes) | Two top-level or two nested classes of the same name in one scope/package (JLS 8.1, 7.6) | `Rename`, `Navigate to duplicate` |
| JAVA-ERR-088 | MUST | `Class '{0}' is public, should be declared in a file named '{0}.java'` | class header | Public top-level class in a file of another name (JLS 7.6) | rename class, rename file, make class non-public, move to new file |
| JAVA-ERR-089 | MUST | `Class '{0}' must either be declared abstract or implement abstract method '{1}' in '{2}'` | class header (enum constants: the constant) | A concrete class does not implement an inherited abstract method (JLS 8.1.1.1, 8.4.8). Variants: `Class '{0}' must implement abstract method '{1}' in '{2}'` (anonymous class) and `Enum constant '{0}' must implement abstract method '{1}' in '{2}'` | `Implement methods` (opens the chooser), `Make '{class}' abstract` (not for anonymous/enum) |
| JAVA-ERR-090 | MUST | `'{0}' is abstract; cannot be instantiated` | the whole `new` expression | `new Abstract()` or `new Interface()` without body (JLS 15.9.1) | `Make '{0}' not abstract` (if no abstract methods), `Implement abstract methods in anonymous class` (adds `{}` body with stubs) |
| JAVA-ERR-091 | MUST | `Enum types cannot be instantiated` | the `new` expression | JLS 8.9 | |
| JAVA-ERR-092 | MUST | `Method does not override method from its superclass` | the `@Override` annotation | `@Override` on a method that overrides/implements nothing (JLS 9.6.4.4) | `Remove '@Override' annotation`; pull-up-abstract fixes (`Make '{method}' abstract in superclass`, ...) |
| JAVA-ERR-093 | MUST | `'{0}' cannot override '{1}' in '{2}'; overridden method is final` | method header | JLS 8.4.8.1 | `Make '{method}' not final` |
| JAVA-ERR-094 | MUST | `{0}; cannot reduce visibility from '{2}' to '{1}'` (`{0}` is `'m()' in 'B' clashes with 'm()' in 'A'`) | method header (or class header when inherited) | Overrider has weaker access (JLS 8.4.8.3) | `Make '{m}' public/...` (change modifier), implement methods for inherited |
| JAVA-ERR-095 | SHOULD | `Static method '{0}' in '{1}' cannot override instance method '{2}' in '{3}'` / `Instance method '{0}' in '{1}' cannot override static method '{2}' in '{3}'` | member header | JLS 8.4.8 | remove `static` / make super method static |
| JAVA-ERR-096 | SHOULD | `{0}; methods have unrelated return types` / `{0}; incompatible return type` / `{0}; overridden method does not throw '{1}'` | method header | Override clashes (JLS 8.4.5, 8.4.8.3) | change return type, add `throws` |
| JAVA-ERR-097 | SHOULD | `{0}; both methods have same erasure` (variants `, yet neither overrides the other`, `, yet neither hides the other`) | member header | JLS 8.4.2 | |
| JAVA-ERR-098 | MUST | `Cannot inherit from final class '{0}'` (choice forms: `final class`, `enum`, `record`, ...) | the reference in extends | JLS 8.1.4 | `Make '{0}' not final` |
| JAVA-ERR-099 | MUST | `Interface expected here` | the reference in implements (or interface extends) | `implements ClassName` | `Change 'implements' to 'extends'` |
| JAVA-ERR-100 | MUST | `No interface expected here` | the reference in extends | `extends InterfaceName` for a class | `Change 'extends' to 'implements'` |
| JAVA-ERR-101 | MUST | `Cyclic inheritance involving '{0}'` | class header | JLS 8.1.4 | |
| JAVA-ERR-102 | MUST | `Method return type missing or constructor name does not match class name` (when the name differs from the class: `Method return type missing`) | method name identifier | A method without return type whose name is not the class name (JLS 8.4, 8.8) | `Rename method to '{class}'`, `Make method return '{determined type}'` |
| JAVA-ERR-103 | MUST | `Abstract methods cannot have a body` / `Interface abstract methods cannot have a body` / `Native methods cannot have a body` | method header | JLS 8.4.3, 9.4 | `Make '{m}' not abstract`, `Delete method body`, push down |
| JAVA-ERR-104 | MUST | `Method body expected` / `Method body or 'abstract' modifier expected` | method header | Non-abstract, non-native method with no body | `Add method body`, `Make '{m}' abstract`... |
| JAVA-ERR-105 | SHOULD | `Abstract method in non-abstract class` | the method header | | make class abstract |
| JAVA-ERR-106 | SHOULD | `Modifier '{0}' not allowed here` / `Repeated modifier '{0}'` / `Illegal combination of modifiers '{0}' and '{1}'` | the offending modifier keyword | JLS 8.1.1, 8.3.1, 8.4.3, 9.x modifier tables | `Make '{x}' package-private` (change modifier) / delete modifier |
| JAVA-ERR-107 | SHOULD | `Call to 'super' must be first statement in constructor body` (also `this`; since Java 22 preview: statements before) | the call | JLS 8.8.7.1 | |
| JAVA-ERR-108 | SHOULD | `Recursive constructor call` | the call | `this(..)` cycle | |
| JAVA-ERR-109 | SHOULD | `Constructor is not allowed in interface` / `Class initializer is not allowed in interface` | | | |
| JAVA-ERR-110 | SHOULD | `Call to '{0}' only allowed in constructor body` / `Call to '{0}' must be top-level statement in constructor body` / `Only one explicit constructor call allowed in constructor` | the call | `this(..)`/`super(..)` outside a constructor, nested, or repeated | |
| JAVA-ERR-111 | SHOULD | `Accessing static field from enum constructor is not allowed` (choice text: `Accessing {static field|enum constant} from enum {constructor|instance field initializer|instance initializer} is not allowed`) | the access | JLS 8.9.2 | |
| JAVA-ERR-112 | SHOULD | `Class '{0}' clashes with package of same name` / `Package '{0}' clashes with class of same name` | name | | |
| JAVA-ERR-113 | SHOULD | `'{0}' is already defined in this compilation unit` | class name | A top-level class vs a single-type import of the same simple name (JLS 7.5.1) | |
| JAVA-ERR-114 | SHOULD | `'{0}' is already defined in a single-type import` | import | | |
| JAVA-ERR-115 | LATER | Sealed/permits family: `Sealed class must have subclasses`, `Sealed class permits clause must contain all subclasses`, `Modifier 'sealed', 'non-sealed' or 'final' expected`, `'{0}' is not allowed in the sealed hierarchy`, `Invalid 'permits' clause: '{0}' must directly extend|implement '{2}'` | class name / reference | JLS 8.1.6, 8.1.1.2 | `Add modifiers`, `Fill permits list` |
| JAVA-ERR-116 | LATER | Records (full list in Appendix A): `Record has no header declared`, `Instance field is not allowed in records`, accessor and canonical constructor rules | | JLS 8.10 | |
| JAVA-ERR-117 | LATER | Interface/default methods: `Extension methods can only be used within an interface`, `Extension method should have a body`, `Default method '{0}' overrides a member of 'java.lang.Object'` | | JLS 9.4 | |

---

## 4.7 Statements, expressions, switch, patterns, lambdas

| ID | Pri | Message | Range | Trigger | Fixes |
|---|---|---|---|---|---|
| JAVA-ERR-118 | MUST | `Not a statement` | the expression statement | `x + 1;`, `"a";`, a bare name (JLS 14.8) | |
| JAVA-ERR-119 | SHOULD | `Declaration not allowed here` | the declaration | `if (x) int y = 1;` | |
| JAVA-ERR-120 | MUST | `'switch' statement does not cover all possible input values` / `'switch' expression does not cover all possible input values` | the `switch` keyword for statement (the selector expression for expressions) | Exhaustiveness (JLS 14.11.1.1, 15.28.1.1) for enhanced switch statements and all switch expressions | `Create missing 'switch' branches`, `Add 'default' branch`, `Unwrap`... |
| JAVA-ERR-121 | SHOULD | `Duplicate label '{0}'` / `Duplicate default label` / `Label is dominated by a preceding case label '{0}'` | the label | JLS 14.11.1 | remove / reorder |
| JAVA-ERR-122 | SHOULD | `Constant expression required` | the label expression | non-constant case label | |
| JAVA-ERR-123 | SHOULD | `Selector type of '{0}' is not supported` | selector | JLS 14.11 | |
| JAVA-ERR-124 | SHOULD | `Pattern '{0}' is not exhaustive on '{1}'`, `Deconstruction pattern can only be applied to a record, '{0}' is not a record`, `Incorrect number of nested patterns: expected {0} but found {1}` | pattern | JLS 14.30 | |
| JAVA-ERR-125 | SHOULD | `Switch expression should produce a result in all execution paths` / `Switch expression rule should produce a result in all execution paths` / `'switch' expression does not have any result expressions` | switch keyword / rule | JLS 15.28.1 | |
| JAVA-ERR-126 | SHOULD | `Integer number too large` / `Long number too large` / `Floating point number too large|small` / `Malformed floating point literal` / `Illegal underscore` | the literal | JLS 3.10.1-3.10.2 | |
| JAVA-ERR-127 | SHOULD | `Too many characters in character literal`, `Unclosed character literal`, `Empty character literal`, `Illegal escape character in character literal|string literal`, `Line end not allowed in string literals`, `Unclosed text block` | literal / escape range | JLS 3.10.4-3.10.6 | `Convert to string literal` for char with several characters |
| JAVA-ERR-128 | SHOULD | `{0} is not a functional interface` | lambda / method-reference expression | Target type has no single abstract method (JLS 9.8) | |
| JAVA-ERR-129 | SHOULD | `Wrong number of parameters in lambda expression: expected {0} but found {1}` | lambda parameter list | | |
| JAVA-ERR-130 | SHOULD | `Incompatible parameter type in lambda expression: expected {0} but found {1}` | the parameter | | |
| JAVA-ERR-131 | SHOULD | `Cannot resolve method '{0}'` for method references (name only, no argument types) / `Cannot resolve constructor '{0}'` | the reference name | `Foo::bar` unresolved | create method |
| JAVA-ERR-132 | SHOULD | `Lambda expression is not expected here` / `Method reference expression is not expected here` | | lambda in a non-functional context | |
| JAVA-ERR-133 | SHOULD | `Bad return type in lambda expression` family (`{0}` reason) | the return expression | JLS 15.27.3 | |
| JAVA-ERR-134 | SHOULD | `Bad type in switch expression: {0} cannot be converted to {1}` | the result expression | A result expression of a switch expression is not compatible with the target type | change type fixes |
| JAVA-ERR-135 | SHOULD | `Qualified new of static class` / `Invalid qualified new` / `No enclosing instance of type '{0}' is in scope` | | `outer.new Static()`; inner class creation without outer instance | |
| JAVA-ERR-136 | SHOULD | `Non-static method cannot be referenced from a static context` / `Static method referenced through non-static qualifier` / `Abstract method '{0}' cannot be accessed directly` | the method reference name | Method-reference qualifier/kind mismatch (JLS 15.13.1) | |


---

## 4.8 Annotations, modules, imports (reference rows)

| ID | Pri | Message | Range | Trigger |
|---|---|---|---|---|
| JAVA-ERR-137 | SHOULD | `'@{0}' not applicable to {1}` (`{1}` = target description such as `type`, `method`, `field`) | the annotation | Annotation `@Target` does not allow this position (JLS 9.6.4.1) |
| JAVA-ERR-138 | SHOULD | `{0} missing but required` (`{0}` = quoted attribute names) | annotation name | Missing element without default (JLS 9.7.1) |
| JAVA-ERR-139 | SHOULD | `Cannot find @interface method '{0}()'` | the attribute name | unknown element |
| JAVA-ERR-140 | SHOULD | `Duplicate annotation` / `Duplicate annotation. The declaration of '{0}' does not have a valid java.lang.annotation.Repeatable annotation` | annotation name | JLS 9.7.5 |
| JAVA-ERR-141 | SHOULD | `Attribute value must be constant` / `...must be a class literal` / `...must be an enum constant` / `Incompatible types. Found: '{1}', required: '{0}'` | the value | JLS 9.7.1 |
| JAVA-ERR-142 | SHOULD | `Annotation type expected` | the reference | |
| JAVA-ERR-143 | LATER | `Container annotation ...` family | | JLS 9.6.3 |
| JAVA-ERR-144 | SHOULD | `Cannot resolve symbol '{0}'` on an **import** reference (see 012; for imports the fix list has only `Add library`/`Move class` style fixes, not `Import class`) | import reference name | Unresolvable single-type import |
| JAVA-ERR-145 | SHOULD | `Class {0} not found` (static import on demand resolving to something that is not a class) / `Field '{0}' is already defined in a single static import` / `Class '{0}' is ambiguous in a single static import` | the import reference | JLS 7.5.3-7.5.4 |
| JAVA-ERR-146 | LATER | Module descriptor errors (`Module not found: {0}`, `Package not found: {0}`, `Duplicate 'requires': {0}`, `Cyclic dependency: {0}`, `Package '{0}' is declared in module '{1}', which does not export it to module '{2}'`, ...) | | JLS 7.7 |
| JAVA-ERR-147 | LATER | Preview/feature gates: `{0} is a preview API and is disabled by default` | | |

---

## 4.9 Syntax errors (parser)

The parser reports an error element with a description; the catalog entry is `Syntax error` whose
message **is the description**. The descriptions (bundle `JavaErrorBundle` and the syntax bundle):

`Identifier expected`, `Identifier or type expected`, `Expression expected`, `';' expected`, `'(' expected`,
`')' expected`, `']' expected`, `'{' expected`, `'}' expected`, `',' expected`, `Parameter expected`,
`'(' or '[' expected`, `Statement expected`, `'while' expected`, `'catch' or 'finally' expected`, `Boolean
expression expected`, `'case', 'default' or '}' expected`, `Expression, block or throw statement expected`,
`Unexpected token`, `'else' without 'if'`, `'catch' without 'try'`, `'finally' without 'try'`, `Unclosed
comment`.

Range rule (`ic: ...JavaErrorKinds` syntax kind): the error element's own range; if the error element is
**empty** and not at end of line, a one-character range at its start is used; if empty at the end of a
line the highlight is placed on the previous token end (zero width extended by the editor to the previous
character) so it is visible. For a missing `;` the highlight is on the position right after the previous
token. Typical shape for `int x = 5` newline `int y;`: `';' expected` shown after `5`.

Fixes on syntax errors: typo fix for keywords (`Replace with keyword` when an identifier is a near-miss of
a keyword), wrap statements of a switch rule in a block for `Expression, block or throw statement
expected`.

- **JAVA-ERR-148** `MUST` Parser recovery yields exactly the descriptions above at the positions above for
  the common cases: missing semicolon, missing closing paren/brace, unbalanced braces at end of file,
  missing identifier after dot, missing expression after operator.
- **JAVA-ERR-149** `MUST` Syntax errors suppress semantic errors inside the erroneous node.
- **JAVA-ERR-150** `SHOULD` Keyword typo quick fix.

---

## 4.10 Acceptance scenarios

**E1 missing return.**

```java
int f(boolean b) {
    if (b) return 1;
}
```
Error `Missing return statement` on the final `}` of `f`. Alt+Enter: `Add 'return' statement`, `Make 'f' return 'void'`.

**E2 incompatible types.**
```java
int n = "text";
```
Error `Incompatible types. Found: 'java.lang.String', required: 'int'` on `"text"`.
```java
String s; s = 5;
```
Error `Incompatible types. Found: 'int', required: 'java.lang.String'` on `s = 5` (whole assignment).

**E3 wrong arguments (one candidate).**
```java
static void take(int a) {}
void g() { take("x"); }
```
Error on `"x"` only: `'take(int)' in 'Main' cannot be applied to '(java.lang.String)'`. With
`take(1, 2)` the error is on `(1, 2)`: `Expected 1 argument but found 2`.

**E4 overloads, none applicable.**
```java
static void put(int a) {}
static void put(String a, int b) {}
void g() { put(true); }
```
Error `Cannot resolve method 'put(boolean)'` on `(true)`.

**E5 unhandled exception.**
```java
void h() { new java.io.FileReader("a"); }
```
Error `Unhandled exception: java.io.FileNotFoundException` on `FileReader` (the class reference), style
unhandled-exception. Alt+Enter: `Add exception to method signature`, `Surround with try/catch`.

**E6 unreachable.**
```java
int k() { return 1; System.out.println(); }
```
Error `Unreachable statement` on `System.out.println();` (whole statement; it is an expression statement,
no keyword). Fix `Delete unreachable statement`.

**E7 uninitialized.**
```java
void m() { int x; System.out.println(x); }
```
Error `Variable 'x' might not have been initialized` on the `x` in `println(x)`; fix `Initialize variable 'x'`.

**E8 abstract instantiation.**
```java
abstract class A {}
void n() { new A(); }
```
Error `'A' is abstract; cannot be instantiated` on `new A()`.

**E9 missing implementation.**
```java
interface I { void run(); }
class C implements I {}
```
Error `Class 'C' must either be declared abstract or implement abstract method 'run()' in 'I'` on `class C
implements I` (class header). Alt+Enter: `Implement methods`, `Make 'C' abstract`.

**E10 final reassign.**
```java
void p() { final int a = 1; a = 2; }
```
Error `Cannot assign a value to final variable 'a'` on `a` (the left side reference). Fix `Make 'a' not final`.

**E11 duplicate variable.**
```java
void q(int a) { int a = 1; }
```
Error `Variable 'a' is already defined in the scope` on the second `a`.

**E12 lambda capture.**
```java
void r() { int c = 0; c++; Runnable x = () -> System.out.println(c); }
```
Error `Variable used in lambda expression should be final or effectively final` on `c` in the lambda.

---

## 4.11 Appendix A: the complete message index

Every error kind of the checker set, keyed by bundle message key, with the message text (shortened to 170
characters) and the range rule extracted from the kind definition ("whole element" means the anchor's whole
text; other entries are narrowed). This index defines the full vocabulary; entries not covered by the
detailed tables above are `LATER` unless a scenario needs them. Where two kinds share one message key only the
first is listed.

- **JAVA-ERR-900** `MUST` Every message key marked in the detailed tables above is implemented with exactly
  this text.
- **JAVA-ERR-901** `LATER` Every other row below is implemented with exactly this text and range.

| Message key | Message | Range |
|---|---|---|
| `insufficient.language.level` | {0} are not supported at language level '{1}' | custom (see source) |
| `illegal.unicode.escape` | Illegal Unicode escape sequence | custom (see source) |
| `illegal.character` | Illegal character: {0} | whole element |
| `syntax.error` | {0} | custom (see source) |
| `preview.api.usage` | {0} is a preview API and is disabled by default | whole element |
| `annotation.not.allowed.here` | Annotations are not allowed here | whole element |
| `annotation.not.allowed.in.permit.list` | Annotations not allowed in 'permits' list | whole element |
| `annotation.not.allowed.on.package` | Package annotations only allowed in a 'package-info.java' file | custom (see source) |
| `annotation.member.may.not.have.throws.list` | 'throws' not allowed on @interface method | custom (see source) |
| `annotation.member.may.not.have.parameters` | @interface members may not have parameters | whole element |
| `annotation.may.not.have.extends.list` | 'extends' not allowed on @interface | custom (see source) |
| `annotation.cannot.be.local` | Local annotations are not allowed | whole element |
| `annotation.not.allowed.var` | 'var' type may not be annotated | whole element |
| `annotation.not.allowed.void` | 'void' type may not be annotated | whole element |
| `annotation.not.allowed.class` | Class literal type may not be annotated | whole element |
| `annotation.not.allowed.ref` | Annotation is not applicable to this kind of reference | whole element |
| `annotation.not.allowed.static` | Static member qualifying type may not be annotated | whole element |
| `annotation.type.expected` | Annotation type expected | whole element |
| `annotation.repeated.target` | Repeated annotation target | whole element |
| `annotation.attribute.name.missing` | Annotation attribute of the form 'name=value' expected | whole element |
| `annotation.attribute.non.class.literal` | Attribute value must be a class literal | whole element |
| `annotation.attribute.non.enum.constant` | Attribute value must be an enum constant | whole element |
| `annotation.attribute.non.constant` | Attribute value must be constant | whole element |
| `annotation.cyclic.element.type` | Cyclic annotation element type | whole element |
| `annotation.member.clash` | @interface member clashes with '{0}' in {1} | name identifier |
| `annotation.member.invalid.type` | Invalid type '{0}' for annotation member | whole element |
| `annotation.attribute.incompatible.type` | Incompatible types. Found: '{1}', required: '{0}' | whole element |
| `annotation.attribute.illegal.array.initializer` | Illegal initializer for '{0}' | whole element |
| `annotation.attribute.duplicate` | Duplicate attribute '{0}' | whole element |
| `annotation.attribute.unknown.method` | Cannot find @interface method '{0}()' | custom (see source); unresolved-ref style |
| `annotation.permits` | 'permits' not allowed on @interface | first token (keyword) of the clause |
| `lambda.not.a.functional.interface` | {0} is not a functional interface | whole element |
| `lambda.no.target.method.found` | No target method found | whole element |
| `lambda.multiple.sam.candidates` | Multiple non-overriding abstract methods found in {0} | whole element |
| `lambda.sealed.functional.interface` | Functional interface cannot be declared as 'sealed' | whole element |
| `lambda.not.expected` | Unexpected lambda expression | whole element |
| `lambda.parameters.inconsistent.var` | Cannot mix 'var' and explicitly typed parameters in lambda expression | whole element |
| `lambda.sealed` | Lambda cannot implement a sealed interface | whole element |
| `lambda.type.inference.failure` | Cannot infer functional interface type | whole element |
| `lambda.sam.generic` | Target method is generic | whole element |
| `lambda.target.not.interface` | Target type of a lambda conversion must be an interface | whole element |
| `lambda.inference.error` | {0} | whole element |
| `lambda.return.type.error` | {0} | whole element |
| `lambda.wrong.number.of.parameters` | Wrong number of parameters in lambda expression: expected {0} but found {1} | parameter list |
| `lambda.incompatible.parameter.types` | Incompatible parameter type in lambda expression: expected {0} but found {1} | whole element |
| `method.reference.sealed` | Method reference cannot implement a sealed interface | whole element |
| `method.reference.not.expected` | Method reference expression is not expected here | whole element |
| `method.reference.raw.constructor` | Raw constructor reference with explicit type parameters for constructor | whole element |
| `method.reference.qualifier.class.unresolved` | Cannot find class {0} | whole element |
| `method.reference.qualifier.wildcard` | Unexpected wildcard | whole element |
| `method.reference.return.type.error` | {0} | whole element |
| `method.reference.inference.error` | {0} | reference name |
| `method.reference.unresolved.constructor` | Cannot resolve constructor '{0}' | reference name |
| `method.reference.unresolved.method` | Cannot resolve method '{0}' | reference name; unresolved-ref style |
| `method.reference.abstract.method` | Abstract method '{0}' cannot be accessed directly | whole element |
| `method.reference.non.static.method.in.static.context` | Non-static method cannot be referenced from a static context | whole element |
| `method.reference.static.method.non.static.qualifier` | Static method referenced through non-static qualifier | whole element |
| `method.reference.static.method.receiver` | Static method referenced through receiver | whole element |
| `method.reference.enclosing.instance.not.in.scope` | An enclosing instance of type {0} is not in scope | whole element |
| `method.reference.parameterized.qualifier` | Parameterized qualifier on static method reference | whole element |
| `annotation.not.applicable` | '@{0}' not applicable to {1} | whole element |
| `annotation.missing.attribute` | {0} missing but required | annotation name |
| `annotation.container.wrong.place` | Container annotation '{0}' must not be present together with the element it contains | whole element |
| `annotation.container.not.applicable` | Container annotation '@{0}' is not applicable to {1} | whole element |
| `annotation.duplicate` | Duplicate annotation | annotation name |
| `annotation.duplicate.non.repeatable` | Duplicate annotation. The declaration of '{0}' does not have a valid java.lang.annotation.Repeatable annotation | annotation name |
| `annotation.duplicate.explained` | Duplicate annotation. {0} | annotation name |
| `annotation.malformed.repeatable.explained` | {0} | whole element |
| `safe.varargs.on.record.component` | @SafeVarargs is not allowed on a record component | whole element |
| `safe.varargs.on.fixed.arity` | @SafeVarargs is not allowed on methods with fixed arity | whole element |
| `safe.varargs.on.non.final.method` | @SafeVarargs is not allowed on non-final instance methods | whole element |
| `override.on.static.method` | Static methods cannot be annotated with @Override | whole element |
| `override.on.non-overriding.method` | Method does not override method from its superclass | whole element |
| `receiver.wrong.context` | Receivers are not allowed outside of method\u2019s parameter list | identifier |
| `receiver.static.context` | The receiver cannot be used in a static context | identifier |
| `receiver.wrong.position` | The receiver should be the first parameter | identifier |
| `receiver.type.mismatch` | The receiver type does not match the enclosing class type | type element |
| `receiver.name.mismatch` | The receiver name does not match the enclosing class type | identifier |
| `class.must.implement.method` | Class '{0}' must implement abstract method '{1}' in '{2}' | custom (see source) |
| `class.inherits.abstract.and.default` | {0} inherits abstract and default for {1} from types {2} and {3} | name identifier |
| `class.inherits.unrelated.defaults` | {0} inherits unrelated defaults for {1} from types {2} and {3} | name identifier |
| `class.already.imported` | '{0}' is already defined in this compilation unit | name identifier |
| `class.duplicate` | Duplicate class: '{0}' | name identifier; file-level |
| `class.clashes.with.package` | Class '{0}' clashes with package of same name | name identifier; file-level |
| `package.clashes.with.class` | Package '{0}' clashes with class of same name | whole element |
| `class.wrong.filename` | Class '{0}' is public, should be declared in a file named '{0}.java' | class header (modifiers w/o annotations .. name/implements list) |
| `class.cyclic.inheritance` | Cyclic inheritance involving '{0}' | class header (modifiers w/o annotations .. name/implements list) |
| `class.reference.list.duplicate` | Duplicate reference to '{0}' in '{1}' list | whole element |
| `class.reference.list.name.expected` | Class name expected | whole element |
| `class.reference.list.inner.private` | '{0}' has private access in '{1}' | whole element |
| `class.reference.list.no.enclosing.instance` | No enclosing instance of type '{0}' is in scope | whole element |
| `class.cannot.extend.multiple.classes` | Class cannot extend multiple classes | whole element |
| `class.extends.interface` | No interface expected here | whole element |
| `class.extends.sealed.local` | Local classes must not extend sealed classes | whole element |
| `class.extends.sealed.not.permitted` | '{0}' is not allowed in the sealed hierarchy | whole element |
| `class.extends.sealed.another.module` | Class is not allowed to extend sealed class from another module | whole element |
| `class.extends.sealed.another.package` | {0} '{1}' from another package not allowed to extend sealed {2} '{3}' in unnamed module | whole element |
| `class.extends.prohibited.class` | Classes cannot directly extend '{0}' | whole element |
| `class.inherits.type.parameter` | Class cannot inherit from its type parameter | whole element |
| `class.extends.final` | Cannot inherit from {1, choice, 1#final class\|2#enum\|3#record\|4#non-abstract value class} '{0}' | whole element |
| `class.anonymous.extends.sealed` | Anonymous classes must not extend sealed classes | base class reference |
| `class.implements.class` | Interface expected here | whole element |
| `class.sealed.no.inheritors` | Sealed class must have subclasses | name identifier |
| `class.sealed.incomplete.permits` | Sealed class permits clause must contain all subclasses | name identifier |
| `class.sealed.inheritor.expected.modifiers.can.be.final` | Modifier 'sealed', 'non-sealed' or 'final' expected | name identifier |
| `class.sealed.inheritor.expected.modifiers` | Modifier 'sealed' or 'non-sealed' expected | name identifier |
| `class.sealed.permits.on.non.sealed` | Invalid 'permits' clause: '{0}' must be sealed | first token (keyword) of the clause |
| `class.not.enclosing` | '{0}' is not an enclosing class | whole element |
| `class.cannot.be.referenced.from.static.context` | '{0}' cannot be referenced from a static context | whole element |
| `class.inheritance.different.type.arguments` | '{0}' cannot be inherited with different type arguments: '{1}' and '{2}' | class header (modifiers w/o annotations .. name/implements list) |
| `class.inheritance.raw.and.generic` | '{0}' cannot be inherited as a raw type and with generic type arguments '{1}' | class header (modifiers w/o annotations .. name/implements list) |
| `class.inheritance.method.clash` | Methods {0} from {1} and {2} from {3} are inherited with the same signature | name identifier |
| `class.generic.extends.exception` | Generic class may not extend 'java.lang.Throwable' | whole element |
| `class.not.accessible` | Cannot access {0} | member header (class/method/field header ranges) |
| `class.initializer.must.complete.normally` | Initializer must be able to complete normally | whole element |
| `class.permitted.must.have.modifier` | All sealed class subclasses must either be final, sealed or non-sealed | whole element |
| `class.permitted.not.direct.subclass` | Invalid 'permits' clause: '{0}' must directly {1, choice, 1#extend\|2#implement} '{2}' | whole element |
| `class.or.package.expected` | Expected class or package | qualifier |
| `value.class.extends.non.abstract` | Value classes may only extend abstract value classes or 'java.lang.Object' | whole element |
| `instantiation.enum` | Enum types cannot be instantiated | whole element |
| `instantiation.abstract` | '{0}' is abstract; cannot be instantiated | whole element |
| `instantiation.local.class.wrong.static.context` | Local class '{0}' cannot be instantiated from a different static context | whole element |
| `record.no.header` | Record has no header declared | name identifier |
| `record.header.regular.class` | Record header declared for non-record | whole element |
| `record.instance.initializer` | Instance initializer is not allowed in records | whole element |
| `record.instance.field` | Instance field is not allowed in records | whole element |
| `record.extends` | 'extends' not allowed on records | first token (keyword) of the clause |
| `record.permits` | 'permits' not allowed on records | first token (keyword) of the clause |
| `record.component.vararg.not.last` | Vararg record component must be the last in the list | whole element |
| `record.component.cstyle.declaration` | C-style array declaration not allowed in record component | custom (see source) |
| `record.component.restricted.name` | Illegal record component name '{0}' | name identifier |
| `record.component.not.initialized` | Record component '{0}' might not be initialized in canonical constructor | name identifier |
| `record.special.method.type.parameters` | {0} cannot have type parameters | whole element |
| `record.special.method.throws` | {0} should not declare a 'throws' clause | first token (keyword) of the clause |
| `record.constructor.stronger.access` | {0} access level cannot be more restrictive than the record access level ('{1}') | name identifier |
| `record.accessor.wrong.return.type` | Incorrect component accessor return type. Expected: '{0}', found: '{1}' | custom (see source) |
| `record.accessor.non.public` | Record component accessor must be 'public' | name identifier |
| `record.no.constructor.call.in.non.canonical` | Non-canonical record constructor must delegate to another constructor | name identifier |
| `record.canonical.constructor.wrong.parameter.type` | Incorrect parameter type for record component '{0}'. Expected: '{1}', found: '{2}' | type element |
| `record.canonical.constructor.wrong.parameter.name` | Canonical constructor parameter names must match record component names. Expected: '{0}', found: '{1}' | name identifier |
| `vararg.not.last.parameter` | Vararg parameter must be the last in the list | whole element |
| `vararg.cstyle.array.declaration` | C-style array declaration not allowed in vararg parameter | custom (see source) |
| `enum.extends` | 'extends' not allowed on enum | first token (keyword) of the clause |
| `enum.permits` | 'permits' not allowed on enum | first token (keyword) of the clause |
| `enum.constant.illegal.access.in.constructor` | Accessing {0, choice, 1#static field\|2#enum constant} from enum {1, choice, 1#constructor\|2#instance field initializer\|3#instance initializer} is not allowed | whole element |
| `enum.constant.modifier` | No modifiers allowed for enum constants | whole element |
| `interface.class.initializer` | Class initializer is not allowed in interface | whole element |
| `interface.constructor` | Constructor is not allowed in interface | whole element |
| `interface.implements` | 'implements' not allowed on interface | first token (keyword) of the clause |
| `interface.extends.class` | Interface expected here | whole element |
| `class.implicit.no.main.method` | Compact source file contains no 'main' method | custom (see source); file-level |
| `class.implicit.invalid.file.name` | The name of a compact source file is not a valid identifier | custom (see source); file-level |
| `class.implicit.initializer` | Initializers are not allowed in compact source files | whole element |
| `class.implicit.package.statement` | Package statement is not allowed in compact source files | whole element |
| `identifier.restricted` | '{0}' is a restricted identifier and cannot be used for type declarations | whole element |
| `type.parameter.extends.interface.expected` | Interface expected here | whole element |
| `type.parameter.cannot.be.followed.by.other.bounds` | Type parameter cannot be followed by other bounds | whole element |
| `type.parameter.on.enum` | Enum may not have type parameters | whole element |
| `type.parameter.on.annotation` | @interface may not have type parameters | whole element |
| `type.parameter.on.annotation.member` | @interface members may not have type parameters | whole element |
| `type.parameter.incompatible.upper.bounds` | Type parameter {0} has incompatible upper bounds: {1} | call: method name; new: class reference; member: header; reference: name |
| `type.parameter.type.not.within.extend.bound` | Type parameter '{0}' is not within its bound; should extend '{1}' | whole element |
| `type.parameter.type.not.within.implement.bound` | Type parameter '{0}' is not within its bound; should implement '{1}' | whole element |
| `type.parameter.absent.class` | Type '{0}' does not have type parameters | whole element |
| `type.parameter.absent.method` | Method '{0}' does not have type parameters | whole element |
| `type.parameter.count.mismatch` | Wrong number of type arguments: {0}; required: {1} | custom (see source) |
| `method.duplicate` | '{0}' is already defined in '{1}' | method header (modifiers w/o annotations .. throws list; whole method if no body) |
| `method.no.parameter.list` | Parameter list expected | name identifier |
| `method.throws.class.name.expected` | Class name expected | whole element |
| `method.interface.body` | Interface abstract methods cannot have a body | method header (modifiers w/o annotations .. throws list; whole method if no body) |
| `method.abstract.body` | Abstract methods cannot have a body | method header (modifiers w/o annotations .. throws list; whole method if no body) |
| `method.abstract.in.non.abstract.class` | Abstract method in non-abstract class | whole element |
| `method.native.body` | Native methods cannot have a body | method header (modifiers w/o annotations .. throws list; whole method if no body) |
| `method.static.in.interface.should.have.body` | Static methods in interfaces should have a body | method header (modifiers w/o annotations .. throws list; whole method if no body) |
| `method.private.in.interface.should.have.body` | Private methods in interfaces should have a body | method header (modifiers w/o annotations .. throws list; whole method if no body) |
| `method.default.should.have.body` | Extension method should have a body | method header (modifiers w/o annotations .. throws list; whole method if no body) |
| `method.default.in.class` | Extension methods can only be used within an interface | method header (modifiers w/o annotations .. throws list; whole method if no body) |
| `method.default.overrides.object.member` | Default method '{0}' overrides a member of 'java.lang.Object' | name identifier |
| `method.should.have.body` | Method body expected | method header (modifiers w/o annotations .. throws list; whole method if no body) |
| `method.should.have.body.or.abstract` | Method body or 'abstract' modifier expected | method header (modifiers w/o annotations .. throws list; whole method if no body) |
| `method.static.overrides.instance` | Static method '{0}' in '{1}' cannot override instance method '{2}' in '{3}' | member header (class/method/field header ranges) |
| `method.instance.overrides.static` | Instance method '{0}' in '{1}' cannot override static method '{2}' in '{3}' | member header (class/method/field header ranges) |
| `method.overrides.final` | '{0}' cannot override '{1}' in '{2}'; overridden method is final | method header (modifiers w/o annotations .. throws list; whole method if no body) |
| `method.inheritance.weaker.privileges` | {0}; cannot reduce visibility from '{2}' to '{1}' | custom (see source) |
| `method.generic.same.erasure` | {0}; both methods have same erasure | member header (class/method/field header ranges) |
| `method.inheritance.clash.unrelated.return.types` | {0}; methods have unrelated return types | class header (modifiers w/o annotations .. name/implements list) |
| `method.missing.return.type` | Method return type missing or constructor name does not match class name | name identifier |
| `method.missing.return.type.not.constructor` | Method return type missing | name identifier |
| `constructor.ambiguous.implicit.call` | Ambiguous implicit constructor call: both '{0}' and '{1}' match | member header (class/method/field header ranges) |
| `constructor.no.default` | There is no no-arg constructor available in '{0}' | member header (class/method/field header ranges) |
| `constructor.in.implicit.class` | Explicit constructor in implicitly declared class of a compact source file is not allowed | member header (class/method/field header ranges) |
| `exception.unhandled` | Unhandled {1, choice, 0#exception\|2#exceptions}: {0} | custom (see source); unhandled-exception style |
| `exception.must.be.disjoint` | Types in multi-catch must be disjoint: '{0}' is a subclass of '{1}' | whole element |
| `exception.already.caught` | Exception '{0}' has already been caught | whole element |
| `exception.never.thrown.try` | Exception '{0}' is never thrown in the corresponding try block | whole element |
| `exception.never.thrown.try.multi` | Exception '{0}' is never thrown in the corresponding try block | whole element |
| `type.incompatible` | Incompatible types. Found: '{1}', required: '{0}' | custom (see source) |
| `type.void.illegal` | Illegal type: 'void' | whole element |
| `type.void.not.allowed` | 'void' type is not allowed here | whole element |
| `type.inaccessible` | '{0}' is inaccessible from here | whole element |
| `type.unknown.class` | Unknown class: '{0}' | whole element |
| `type.argument.primitive` | Type argument cannot be of a primitive type | whole element |
| `type.argument.not.allowed` | Reference parameters are not allowed here | whole element |
| `type.argument.on.raw.type` | Type arguments given on a raw type | whole element |
| `type.argument.on.raw.method` | Type arguments given on a raw method | whole element |
| `type.argument.in.permits.list` | Generics are not allowed in permits list | whole element |
| `type.wildcard.not.expected` | No wildcard expected | whole element |
| `type.wildcard.may.be.used.only.as.reference.parameters` | Wildcards may only be used as reference parameters | whole element |
| `type.wildcard.cannot.be.instantiated` | Wildcard type '{0}' cannot be instantiated directly | whole element |
| `type.restricted.identifier` | Illegal reference to restricted type '{0}' | reference name |
| `foreach.not.applicable` | Foreach not applicable to type '{0}' | whole element |
| `lvti.no.initializer` | Cannot infer type: 'var' on variable without an initializer | type element |
| `lvti.self.referenced` | Cannot infer type for '{0}', it is used in its own variable initializer | whole element |
| `lvti.compound` | 'var' is not allowed in a compound declaration | whole element |
| `lvti.void` | Cannot infer type: variable initializer is 'void' | type element |
| `lvti.null` | Cannot infer type: variable initializer is 'null' | type element |
| `lvti.lambda` | Cannot infer type: lambda expression requires an explicit target type | type element |
| `lvti.method.reference` | Cannot infer type: method reference requires an explicit target type | type element |
| `lvti.array` | 'var' is not allowed as an element type of an array | type element |
| `label.without.statement` | Label without statement | label identifier |
| `label.duplicate` | Label '{0}' already in use | label identifier |
| `label.unresolved` | Undefined label: '{0}' | whole element |
| `label.must.be.loop` | 'continue' target must be a loop label: '{0}' | whole element |
| `break.outside.switch.or.loop` | 'break' outside of switch or loop | whole element |
| `break.out.of.switch.expression` | Break out of switch expression is not allowed | whole element |
| `continue.outside.loop` | 'continue' statement outside of loop | whole element |
| `continue.out.of.switch.expression` | Continue out of switch expression is not allowed | whole element |
| `yield.unexpected` | 'yield' outside of a switch expression | whole element |
| `yield.void` | Expression type should not be 'void' | whole element |
| `catch.type.parameter` | Cannot catch type parameters | whole element |
| `array.illegal.initializer` | Illegal initializer for '{0}' | whole element |
| `array.initializer.not.allowed` | Array initializer is not allowed here | whole element |
| `array.type.expected` | Array type expected; found: '{0}' | whole element |
| `array.generic` | Generic array creation not allowed | whole element |
| `array.empty.diamond` | Array creation with '<>' not allowed | whole element |
| `array.type.arguments` | Array creation with type arguments not allowed | whole element |
| `array.too.many.dimensions` | Too many array dimensions | whole element |
| `pattern.type.pattern.expected` | Type pattern expected | whole element |
| `pattern.deconstruction.variable` | Identifier is not allowed here | whole element |
| `pattern.deconstruction.annotation` | Annotations are not allowed in deconstruction pattern types | whole element |
| `pattern.unsafe.cast` | '{0}' cannot be safely cast to '{1}' | whole element |
| `pattern.not.exhaustive` | Pattern '{0}' is not exhaustive on '{1}' | whole element |
| `pattern.deconstruction.requires.record` | Deconstruction pattern can only be applied to a record, '{0}' is not a record | whole element |
| `pattern.cannot.infer.type` | Cannot infer pattern type: {0} | whole element |
| `pattern.deconstruction.count.mismatch` | Incorrect number of nested patterns: expected {0} but found {1} | custom (see source) |
| `pattern.instanceof.supertype` | Pattern type '{0}' is a supertype of expression type '{1}' | custom (see source) |
| `pattern.instanceof.equals` | Pattern type '{0}' is the same as expression type | custom (see source) |
| `pattern.expected.class.or.array.type` | Class or array type is required | whole element |
| `instanceof.type.parameter` | Class or array expected | whole element |
| `instanceof.illegal.generic.type` | Illegal generic type for instanceof | whole element |
| `instanceof.unsafe.cast` | '{0}' cannot be safely cast to '{1}' | whole element |
| `cast.inconvertible` | Inconvertible types; cannot cast '{0}' to '{1}' | whole element |
| `cast.intersection.not.interface` | Interface expected here | whole element |
| `cast.intersection.unexpected.type` | Unexpected type: class is expected | whole element |
| `cast.intersection.repeated.interface` | Repeated interface | whole element |
| `cast.intersection.inheritance.clash` | {0} cannot be inherited with different arguments: {1} and {2} | whole element |
| `resource.declaration.or.variable.expected` | Declaration, final, or effectively final variable expected | whole element |
| `switch.rule.should.produce.result` | Switch expression rule should produce a result in all execution paths | custom (see source) |
| `switch.expression.no.result` | 'switch' expression does not have any result expressions | custom (see source) |
| `switch.expression.should.produce.result` | Switch expression should produce a result in all execution paths | custom (see source) |
| `switch.expression.cannot.be.void` | Target type for switch expression cannot be void | custom (see source) |
| `switch.expression.incompatible.type` | Bad type in switch expression: {0} cannot be converted to {1} | whole element |
| `switch.label.expected` | Statement must be prepended with a case label | whole element |
| `switch.different.case.kinds` | Different 'case' kinds used in 'switch' | custom (see source) |
| `switch.selector.type.invalid` | Selector type of '{0}' is not supported | whole element |
| `switch.null.type.incompatible` | 'null' cannot be converted to '{0}' | whole element |
| `switch.label.qualified.enum` | An enum switch case label must be the unqualified name of an enumeration constant | whole element |
| `switch.label.constant.expected` | Constant expression required | whole element |
| `switch.label.unexpected` | Constant expression, pattern or null is required | whole element |
| `switch.default.label.contains.case` | The label for the default case must only use the 'default' keyword, without 'case' | whole element |
| `switch.label.pattern.expected` | Pattern expected for switch selector type '{0}' | whole element |
| `switch.label.duplicate` | Duplicate label '{0}' | whole element |
| `switch.fallthrough.to.pattern` | Illegal fall-through to a pattern | whole element |
| `switch.multiple.labels.with.pattern.variables` | Multiple switch labels are permitted for a switch labeled statement group only if none of them declare any pattern variables | whole element |
| `switch.default.null.order` | Invalid case label order: 'null' must be first and 'default' must be second | whole element |
| `switch.default.label.not.allowed` | Default label not allowed here: 'default' can only be used as a single case label or paired only with 'null' | whole element |
| `switch.null.label.not.allowed` | Invalid case label combination: 'null' can only be used as a single case label or paired only with 'default' | whole element |
| `switch.label.combination.constants.and.patterns` | Invalid case label combination: a case label must consist of either a list of case constants or a single case pattern | whole element |
| `switch.label.combination.constants.and.patterns.unnamed` | Invalid case label combination: a case label must consist of either a list of case constants or a list of case patterns | whole element |
| `switch.label.multiple.patterns` | Invalid case label combination: a case label must not consist of more than one case pattern | whole element |
| `switch.label.multiple.patterns.unnamed` | Invalid case label combination: multiple patterns are allowed only if none of them declare any pattern variables | whole element |
| `switch.dominance.violation` | Label is dominated by a preceding case label '{0}' | whole element |
| `switch.unconditional.pattern.and.default` | 'switch' has both an unconditional pattern and a default label | whole element |
| `switch.default.and.boolean` | 'switch' has all boolean values and a default label | whole element |
| `switch.unconditional.pattern.and.boolean` | 'switch' has all boolean values and an unconditional pattern | whole element |
| `switch.empty` | 'switch' {0} does not have any case clauses | custom (see source) |
| `switch.incomplete` | 'switch' {0} does not cover all possible input values | custom (see source) |
| `expression.expected` | Expression expected | whole element |
| `expression.super.unqualified.default.method` | Unqualified super reference is not allowed in extension method | whole element |
| `expression.super.dot.expected` | '.' expected | custom (see source) |
| `expression.super.not.enclosing.class` | '{0}' is not an enclosing class | whole element |
| `expression.super.bad.qualifier.redundant.extended` | Bad type qualifier in default super call: redundant interface {0} is extended by {1} | whole element |
| `expression.super.bad.qualifier.method.overridden` | Bad type qualifier in default super call: method {0} is overridden in {1} | qualifier |
| `expression.super.no.enclosing.instance` | No enclosing instance of type '{0}' is in scope | whole element |
| `expression.qualified.class.expected` | Class name expected here | whole element |
| `expression.class.type.parameter` | Cannot access class object of a type parameter | whole element |
| `expression.class.parameterized.type` | Cannot access class object of a parameterized type | whole element |
| `assignment.declared.outside.guard` | Cannot assign a value to variable '{0}', because it is declared outside the guard | whole element |
| `assignment.to.final.variable` | Cannot assign a value to final variable '{0}' | whole element |
| `lvalue.variable.expected` | Reference to variable expected on left-hand side of assignment | whole element |
| `unary.operation.variable.expected` | Reference to variable expected as target of unary operation | whole element |
| `binary.operator.not.applicable` | Operator '{0}' cannot be applied to '{1}', '{2}' | custom (see source) |
| `unary.operator.not.applicable` | Operator '{0}' cannot be applied to '{1}' | whole element |
| `new.expression.diamond.not.applicable` | Diamond operator is not applicable to non-parameterized types | whole element |
| `new.expression.qualified.malformed` | Invalid qualified new | whole element |
| `new.expression.qualified.static.class` | Qualified new of static class | whole element |
| `new.expression.qualified.anonymous.implements.interface` | Unexpected qualifier for anonymous class implementing interface | whole element |
| `new.expression.qualified.qualified.class.reference` | Qualified class reference is not allowed in qualified new | whole element |
| `new.expression.diamond.not.allowed` | Diamond operator is not allowed here | whole element |
| `new.expression.diamond.anonymous.inner.non.private` | Cannot use '<>' due to a non-private method which doesn't override or implement a method from a supertype | whole element |
| `new.expression.anonymous.implements.interface.with.type.arguments` | Anonymous class implements interface; cannot have type arguments | whole element |
| `new.expression.unresolved.constructor` | Cannot resolve constructor '{0}' | argument list (including parentheses) |
| `new.expression.type.parameter` | Type parameter '{0}' cannot be instantiated directly | whole element |
| `reference.member.before.constructor` | Cannot reference '{0}' before superclass constructor is called | call: method name; new: class reference; member: header; reference: name |
| `reference.type.argument.static.class` | Type arguments are not allowed here because class '{0}' is static | whole element |
| `reference.type.needs.type.arguments` | Improperly formed type: '{0}' needs type arguments because its qualifier has type arguments | whole element |
| `reference.local.class.other.switch.branch` | Local class '{0}' cannot be referenced from another switch branch | custom (see source); unresolved-ref style |
| `reference.field.forward` | Cannot read value of field '{0}' before the field's definition | whole element |
| `reference.field.self` | Cannot read value of field '{0}' from inside the field's definition | whole element |
| `reference.enum.forward` | Cannot refer to enum constant '{0}' before its definition | whole element |
| `reference.enum.self` | Cannot refer to enum constant '{0}' from inside its own definition | whole element |
| `reference.qualifier.not.expression` | Qualifier must be an expression | custom (see source); unresolved-ref style |
| `reference.qualifier.primitive` | Cannot access fields on '{0}' type | reference name; unresolved-ref style |
| `incomplete.project.state.pending.reference` | Not resolved until the project is fully loaded | custom (see source); pending-ref style |
| `reference.unresolved` | Cannot resolve symbol '{0}' | reference name; unresolved-ref style |
| `reference.implicit.class` | Implicitly declared class of a compact source file '{0}' cannot be referenced | reference name; unresolved-ref style |
| `reference.class.in.default.package` | Class '{0}' is in the default package | reference name; unresolved-ref style |
| `reference.ambiguous` | Reference to '{0}' is ambiguous, both '{1}' and '{2}' match | reference name; unresolved-ref style |
| `reference.non.static.from.static.context` | Non-static {0} '{1}' cannot be referenced from a static context | reference name; unresolved-ref style |
| `reference.outer.type.parameter.from.static.context` | '{0}' cannot be referenced from a static context | custom (see source); unresolved-ref style |
| `reference.select.from.type.parameter` | Cannot select from a type parameter | whole element |
| `reference.package.not.found` | Package not found: {0} | whole element |
| `statement.case.outside.switch` | Case statement outside switch | whole element |
| `statement.invalid` | Invalid statement | whole element |
| `statement.unreachable.loop.body` | Loop condition is always false making the loop body unreachable | whole element |
| `statement.unreachable` | Unreachable statement | custom (see source) |
| `statement.bad.expression` | Not a statement | whole element |
| `statement.declaration.not.allowed` | Declaration not allowed here | whole element |
| `guard.misplaced` | Guard is allowed after patterns only | whole element |
| `guard.evaluated.to.false` | Case label has a guard that is a constant expression with value 'false' | whole element |
| `comment.shebang.java.file` | Shebang mechanism in .java files is not permitted | custom (see source) |
| `comment.unclosed` | Unclosed comment | custom (see source) |
| `literal.illegal.underscore` | Illegal underscore | whole element |
| `literal.hexadecimal.no.digits` | Hexadecimal numbers must contain at least one hexadecimal digit | whole element |
| `literal.binary.no.digits` | Binary numbers must contain at least one binary digit | whole element |
| `literal.integer.too.large` | Integer number too large | whole element |
| `literal.long.too.large` | Long number too large | whole element |
| `literal.floating.malformed` | Malformed floating point literal | whole element |
| `literal.floating.too.large` | Floating point number too large | whole element |
| `literal.floating.too.small` | Floating point number too small | whole element |
| `literal.character.illegal.escape` | Illegal escape character in character literal | custom (see source) |
| `literal.character.too.long` | Too many characters in character literal | whole element |
| `literal.character.empty` | Empty character literal | whole element |
| `literal.character.unclosed` | Unclosed character literal | whole element |
| `literal.string.illegal.escape` | Illegal escape character in string literal | custom (see source) |
| `literal.string.illegal.line.end` | Line end not allowed in string literals | whole element |
| `literal.text.block.unclosed` | Unclosed text block | custom (see source) |
| `literal.text.block.no.new.line` | Illegal text block start: missing new line after opening quotes | custom (see source) |
| `modifier.not.allowed` | Modifier '{0}' not allowed here | whole element |
| `modifier.repeated` | Repeated modifier '{0}' | whole element |
| `modifier.incompatible` | Illegal combination of modifiers '{0}' and '{1}' | whole element |
| `modifier.not.allowed.local.class` | Modifier '{0}' not allowed on local classes | whole element |
| `modifier.not.allowed.non.sealed` | Modifier 'non-sealed' is not allowed on classes that do not have a sealed superclass | whole element |
| `call.super.enum.constructor` | Call to 'super' is not allowed in enum constructor | whole element |
| `call.super.qualifier.not.inner.class` | Qualifier is not allowed because superclass '{0}' is not a non-static inner class | whole element |
| `call.expected` | Method call expected | whole element |
| `call.parsed.as.deconstruction.pattern` | Constant expression, pattern or null is required | whole element |
| `call.static.interface.method.qualifier` | Static method may only be called on its containing interface | whole element |
| `call.formal.varargs.element.type.inaccessible.here` | Formal varargs element type {0} is inaccessible from here | argument list (including parentheses) |
| `call.type.inference.error` | {0} | call: method name; new: class reference; member: header; reference: name |
| `call.wrong.arguments` | '{0}' in '{1}' cannot be applied to '{2}' | custom (see source) |
| `call.unresolved` | Cannot resolve method '{0}' | argument list (including parentheses) |
| `call.unresolved.name` | Cannot resolve method '{0}' | call: method name; new: class reference; member: header; reference: name; unresolved-ref style |
| `call.ambiguous` | Ambiguous method call: both '{0}' and '{1}' match | argument list (including parentheses) |
| `call.ambiguous.no.match` | Cannot resolve method '{0}' in '{1}' | call: method name; new: class reference; member: header; reference: name; unresolved-ref style |
| `call.qualifier.primitive` | Cannot call methods on '{0}' type | call: method name; new: class reference; member: header; reference: name; unresolved-ref style |
| `call.direct.abstract.method.access` | Abstract method '{0}' cannot be accessed directly | whole element |
| `call.constructor.must.be.first.statement` | Call to '{0}' must be first statement in constructor body | whole element |
| `call.constructor.only.allowed.in.constructor` | Call to '{0}' only allowed in constructor body | whole element |
| `call.constructor.must.be.top.level.statement` | Call to '{0}' must be top-level statement in constructor body | whole element |
| `call.constructor.duplicate` | Only one explicit constructor call allowed in constructor | whole element |
| `call.constructor.recursive` | Recursive constructor call | whole element |
| `call.constructor.record.in.canonical` | Canonical constructor cannot delegate to another constructor | whole element |
| `call.member.before.constructor` | Cannot call '{0}' before superclass constructor is called | call: method name; new: class reference; member: header; reference: name |
| `string.template.void.not.allowed.in.embedded` | Expression with the 'void' type is not allowed as a string template embedded expression | whole element |
| `string.template.processor.missing` | Processor missing from string template expression | whole element |
| `string.template.raw.processor` | Raw processor type is not allowed: {0} | whole element |
| `access.private` | '{0}' has private access in '{1}' | call: method name; new: class reference; member: header; reference: name |
| `access.protected` | '{0}' has protected access in '{1}' | call: method name; new: class reference; member: header; reference: name |
| `access.package.local` | '{0}' is not public in '{1}'. Cannot be accessed from outside package | call: method name; new: class reference; member: header; reference: name |
| `access.generic.problem` | Cannot access '{0}' in '{1}' | call: method name; new: class reference; member: header; reference: name |
| `module.import.not.allowed` | Module import is not allowed | whole element |
| `import.single.class.conflict` | '{0}' is already defined in a single-type import | whole element |
| `import.single.static.class.already.defined` | Class '{0}' is already defined in a single static import | whole element |
| `import.single.static.class.ambiguous` | Class '{0}' is ambiguous in a single static import | whole element |
| `import.single.static.field.already.defined` | Field '{0}' is already defined in a single static import | whole element |
| `import.single.static.field.ambiguous` | Field '{0}' is ambiguous in a single static import | whole element |
| `import.static.on.demand.resolves.to.class` | Class {0} not found | reference name |
| `import.list.extra.semicolon` | Extra semicolons between import statements are not allowed | whole element |
| `underscore.identifier` | Since Java 9, '_' is a keyword and may not be used as an identifier | whole element |
| `underscore.identifier.unnamed` | Using '_' as a reference is not allowed | whole element |
| `underscore.identifier.lambda` | Use of '_' as a lambda parameter name is not allowed | whole element |
| `unnamed.variable.brackets` | Brackets are not allowed after an unnamed variable declaration | custom (see source) |
| `unnamed.variable.without.initializer` | Unnamed variable declaration must have an initializer | name identifier |
| `unnamed.field.not.allowed` | Unnamed field is not allowed | name identifier |
| `unnamed.method.parameter.not.allowed` | Unnamed method parameter is not allowed | name identifier |
| `unnamed.variable.not.allowed.in.this.context` | Unnamed variable declaration is not allowed in this context | name identifier |
| `return.outside.switch.expression` | Return outside of enclosing switch expression | whole element |
| `return.compact.constructor` | 'return' statement is not allowed in compact constructor | whole element |
| `return.outside.method` | Return outside method | whole element |
| `return.value.missing` | Missing return value | whole element |
| `return.from.constructor` | Cannot return a value from a constructor | whole element |
| `return.missing` | Missing return statement | whole element |
| `return.from.void.method` | Cannot return a value from a method with void result type | whole element |
| `return.before.explicit.constructor.call` | 'return' not allowed before '{0}' call | whole element |
| `field.not.initialized` | Field '{0}' might not have been initialized | field header (modifiers w/o annotations .. name) |
| `field.initialized.before.constructor.call` | Cannot assign initialized field '{0}' before superclass constructor is called | whole element |
| `variable.not.initialized` | Variable '{0}' might not have been initialized | whole element |
| `variable.already.defined` | Variable '{0}' is already defined in the scope | name identifier |
| `variable.already.assigned` | Variable '{0}' might already have been assigned to | whole element |
| `variable.already.assigned.constructor` | Cannot assign final field '{0}' {1, choice, 1#before\|2#after} chained constructor call | whole element |
| `variable.already.assigned.field` | Final field '{0}' is already initialized in another field initializer | whole element |
| `variable.already.assigned.initializer` | Final field '{0}' is already initialized in a class initializer | whole element |
| `variable.assigned.in.loop` | Variable '{0}' might be assigned in a loop | whole element |
| `variable.must.be.final` | Variable '{0}' is accessed from within inner class, needs to be declared final | whole element |
| `variable.must.be.final.resource` | Variable used as a try-with-resources resource should be final or effectively final | whole element |
| `variable.must.be.effectively.final` | Variable '{0}' is accessed from within inner class, needs to be final or effectively final | whole element |
| `variable.must.be.effectively.final.lambda` | Variable used in lambda expression should be final or effectively final | whole element |
| `variable.must.be.effectively.final.guard` | Variable used in guard expression should be final or effectively final | whole element |
| `module.no.package` | A module file should not have a 'package' statement | whole element |
| `module.conflicting.packages` | Package '{0}' exists in another module: {1} | whole element |
| `module.file.wrong.name` | Module declarations only allowed in a 'module-info.java' file | call: method name; new: class reference; member: header; reference: name |
| `module.file.duplicate` | 'module-info.java' already exists in the module | custom (see source) |
| `module.duplicate.requires` | Duplicate 'requires': {0} | whole element |
| `module.duplicate.exports` | Duplicate 'exports': {0} | whole element |
| `module.duplicate.opens` | Duplicate 'opens': {0} | whole element |
| `module.duplicate.uses` | Duplicate 'uses': {0} | whole element |
| `module.duplicate.provides` | Duplicate 'provides': {0} | whole element |
| `module.conflicting.reads` | Module '{0}' reads package '{1}' from both '{2}' and '{3}' | custom (see source) |
| `module.file.wrong.location` | Module declaration should be located in a module's source root | custom (see source) |
| `module.opens.in.weak.module` | 'opens' is not allowed in an open module | whole element |
| `module.service.enum` | The service definition is an enum: {0} | reference name |
| `module.service.alien` | The service implementation must be defined in the same module as the provides directive | reference name |
| `module.not.found` | Module not found: {0} | custom (see source); unresolved-ref style |
| `module.not.on.path` | Module is not in dependencies: {0} | whole element |
| `module.cyclic.dependence` | Cyclic dependency: {0} | whole element |
| `module.duplicate.implementation` | Duplicate implementation: {0} | reference name |
| `module.service.provider.type` | The 'provider' method return type must be a subtype of the service interface type: {0} | reference name |
| `module.service.no.constructor` | The service implementation does not have a public default constructor: {0} | reference name |
| `module.service.abstract` | The service implementation is an abstract class: {0} | reference name |
| `module.service.inner` | The service implementation is an inner class: {0} | reference name |
| `module.service.implementation.type` | The service implementation type must be a subtype of the service interface type, or have a public static no-args 'provider' method | reference name |
| `module.duplicate.exports.target` | Duplicate 'exports' target: {0} | whole element |
| `module.duplicate.opens.target` | Duplicate 'opens' target: {0} | whole element |
| `module.reference.package.not.found` | Package not found: {0} | package reference |
| `module.reference.package.empty` | Package is empty: {0} | package reference |
| `module.access.from.named` | Package '{0}' is declared in module '{1}', which does not export it to module '{2}' | whole element |
| `module.access.from.unnamed` | Package '{0}' is declared in module '{1}', which does not export it to the unnamed module | whole element |
| `module.access.to.unnamed` | Package '{0}' is declared in the unnamed module, but module '{1}' does not read it | whole element |
| `module.access.package.bad.name` | Package '{0}' is declared in module with an invalid name ('{1}') | whole element |
| `module.access.bad.name` | Module '{0}' has an invalid name | whole element |
| `module.access.package.not.in.graph` | Package '{0}' is declared in module '{1}', which is not in the module graph | whole element |
| `module.access.not.in.graph` | Module '{0}' is missing from the module graph | whole element |
| `module.access.package.does.not.read` | Package '{0}' is declared in module '{1}', but module '{2}' does not read it | whole element |
| `module.access.does.not.read` | Module '{0}' fails to read '{1}' | whole element |
| `module.access.jps.dependency.problem` | Module dependency for '{0}' is not specified in project structure | whole element |

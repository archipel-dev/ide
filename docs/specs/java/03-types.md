# 03. Types

Status: specification, clean-room. Semantics are defined by the Java Language Specification (JLS SE 21 numbering,
public); IntelliJ's observable deviations and conventions, read from the Community sources, are marked
**IJ**. Source paths are cited as `ic: <path>`. No code is reproduced.

Primary sources: `ic: java/java-psi-api/src/com/intellij/psi/*Type*.java` (the type model),
`ic: java/java-psi-api/.../util/TypeConversionUtil.java` (conversions), `ic: java/java-psi-api/.../GenericsUtil.java`
(least upper bound, bounds checks), `ic: java/java-psi-impl/.../impl/source/resolve/graphInference/` (inference),
`ic: java/java-psi-impl/.../scope/conflictResolvers/JavaMethodsConflictResolver.java` (overload selection).
Overload resolution mechanics live in `02-resolve.md` section 2.4; this file defines the types it uses.

---

## 3.1 The type model

Every expression, variable, method return and declaration type is a value of one of these kinds. The model
must be a closed sum type with structural equality, hashing and a canonical text form.

| Kind | Meaning | Notes |
|---|---|---|
| **primitive** | `boolean byte short char int long float double` | plus `void` as a distinct "no value" type |
| **null type** | the type of the literal `null` | assignable to every reference type; not denotable |
| **class type** | reference to a class, interface, enum, record or annotation, with a (possibly empty) list of type arguments and an enclosing-type chain for inner classes; or **raw** (no arguments given for a generic class) | resolves lazily to a *class + substitutor* pair; an **unresolved** class type keeps its name and has no target |
| **array type** | component type + dimension | covariant per JLS 4.10.3; `T...` is the *ellipsis* variant used only for the last formal parameter |
| **type variable** | the type of a type parameter (`T`) of a class, method or constructor | bounds are the declared `extends` list (default `Object`) |
| **wildcard** | `?`, `? extends B`, `? super B` | only as a type argument |
| **captured wildcard** | a fresh type variable produced by capture conversion (3.4) with an upper bound and a lower bound | not denotable; printed as `capture of ?` / `capture of ? extends B` |
| **intersection** | `A & B & ...` | from casts, inference, and the lub of unrelated types |
| **disjunction** | `A | B` of a multi-catch parameter | its type is the lub of the alternatives; each alternative is also visible |
| **diamond** | `<>` as a type argument list | placeholder resolved at the creation site (3.8) |
| **lambda / method-reference pseudo type** | the type of an implicitly typed lambda or an inexact method reference *before* a target type is known | used inside applicability and inference; never shown to users |
| **lambda parameter type (unknown)** | the type of an implicit lambda parameter whose target is unresolved | marks "incomplete code" so errors are suppressed |

Class-type resolution result: `(class, substitutor)` where the *substitutor* is a map from the class's
type parameters to types (empty map and "raw" flag for raw types). All member lookups, supertypes and
conversions work on `(class, substitutor)` pairs; a class type with arguments is the pair with those
arguments bound.

**Annotations on types** (type-use annotations, nullability) are carried by types but never affect
conversion and equality; they are used by the nullability analysis (`07-inspections.md`).

- **JAVA-TYP-001** `MUST` A closed type enum with the kinds above, structural equality and hash, and
  canonical text. *Accept:* `List<String>` equals `java.util.List<java.lang.String>` after resolution;
  two class types with different type arguments differ; `T[]` vs `T...` are equal for conversion.
- **JAVA-TYP-002** `MUST` Class types resolve lazily to `(class, substitutor)`; unresolved class types are
  a value, not an error.
- **JAVA-TYP-003** `MUST` Printing: *canonical* (fully qualified, used in error messages `04` 4.1.2),
  *presentable* (simple names, used in UI and in call-unresolved messages) and *internal* (for debugging).
  Captured wildcards print as `capture of ? extends X`; intersections as `A & B`.
- **JAVA-TYP-004** `SHOULD` Type-use annotations stored on types, ignored by equality.

---

## 3.2 Subtyping and conversions

### 3.2.1 Subtyping (JLS 4.10)

`S <: T` (S is a subtype of T) is defined structurally:
- identity; primitive subtyping chain `byte <: short <: int <: long <: float <: double`, `char <: int`;
- every class/interface type is a subtype of `Object`; direct supertypes of a class type `C<A1..An>`: the
  declared superclass and interfaces with the type arguments substituted, and for a raw type the erasures;
  an interface without superinterfaces has direct supertype `Object`;
- class type containment (JLS 4.5.1): `C<A1..An> <: C<B1..Bn>` iff each `Ai` is *contained by* `Bi`:
  equal types, `? extends U` contains any `A <: U`, `? super L` contains any `A :> L`, `?` contains anything;
- arrays: `S[] <: T[]` iff `S <: T` for references; `S[] <: Object, Cloneable, java.io.Serializable`; arrays
  of different primitive types are unrelated;
- type variable `T <: ` each of its bounds; the null type is a subtype of every reference type;
- intersection `A & B <: X` iff some conjunct `<: X`; `X <: A & B` iff `X <: A` and `X <: B`;
- raw/unchecked: `C <: C<..>` is **not** subtyping but is allowed as an *unchecked conversion* (3.2.2).
Supertypes of a parameterized type are computed by *substitution along the inheritance path*: if `class A<T>
extends B<List<T>>` then the supertype `B<List<String>>` of `A<String>` is obtained by substituting
`T := String` in `B<List<T>>`. **IJ**: the transitive closure of supertypes per class is cached per class
(scoped by resolve scope).

### 3.2.2 Assignment and invocation conversions (JLS 5.1-5.3)

`isAssignable(target, source)` is true if the source can be converted to the target in an **assignment
context**; the same relation without constant narrowing is the **invocation (loose)** context; the
**strict** invocation context disallows boxing/unboxing.

Allowed conversions, in the order a checker tries them:
1. identity;
2. widening primitive (JLS 5.1.2): `byte→short,int,long,float,double`, `short→int,long,float,double`,
   `char→int,long,float,double`, `int→long,float,double`, `long→float,double`, `float→double`;
3. widening reference (subtyping), including the null type to any reference type;
4. boxing (5.1.7): `boolean→Boolean`, `byte→Byte`, `short→Short`, `char→Character`, `int→Integer`,
   `long→Long`, `float→Float`, `double→Double`, optionally followed by widening reference (`int → Number`,
   `int → Object`, `int → Comparable<Integer>`);
5. unboxing (5.1.8): the wrapper to its primitive, optionally followed by widening primitive
   (`Integer → long`, `Byte → int`); a *type variable* with a wrapper bound unboxes through its bound;
6. unchecked conversion (5.1.9): a raw type to any parameterization of the same class or a supertype
   (`List → List<String>`, `ArrayList → List<String>`), reported as an *unchecked* warning, never an error;
7. in the **assignment context only**: *constant narrowing* (5.2): a **constant expression** (JLS 15.29) of
   type `byte`, `short`, `char` or `int` whose value fits is assignable to `byte`, `short`, `char` or their
   wrappers (`byte b = 10; Character c = 65;`), but not to `int → Integer` of other sizes (`Short s = 70000`
   fails).
Not allowed implicitly: narrowing primitive (`int → byte` of a non-constant), `long → int`, any primitive ↔
`boolean`, boxing then narrowing (`int → Long`, `int → Short` of non-constant), unboxing then non-widening.

Casting conversion (5.5, `(T) expr`): all of the above plus narrowing primitive, narrowing reference
(downcast, allowed when the two types are *provably related*: one is a subtype of the other, or one is an
interface and the other a non-final class, or erasure-related for parameterized types unless provably
distinct, e.g. `List<String>` to `List<Integer>` is rejected), unchecked cast, and unboxing/boxing
combinations (`Object → int` via `Integer`). **IJ**: the "convertible" check (`areTypesConvertible`) is the
operational definition of cast legality; `instanceof` uses it with the additional rule that a primitive
operand needs the primitive-patterns language level.

String conversion (5.4): in `+` with a `String` operand and in string-switch contexts every operand type
converts to `String`.

- **JAVA-TYP-005** `MUST` Subtyping per 3.2.1 including containment, arrays, intersections, type variables.
- **JAVA-TYP-006** `MUST` Assignment/invocation/casting relations per 3.2.2 with the three contexts.
  *Accept table:*
  - assignable: `long = int`, `Object = int`, `double = Integer`, `Number = int`, `List<String> = ArrayList<String>`,
    `byte = 10`, `char = 65`, `Character = 'a'`, `List<String> = rawList` (unchecked), `Comparable<Integer> = 5`;
  - not assignable: `int = long`, `int = 3.0`, `byte = 200`, `Long = 5`, `Integer = 5L`, `List<Object> = List<String>`,
    `boolean = 0`, `short = (int variable)`.
- **JAVA-TYP-007** `MUST` Unchecked conversion is allowed with a flag the checker turns into a warning, not
  an error.
- **JAVA-TYP-008** `MUST` Cast legality per 3.2.2 (provably distinct parameterizations rejected).
  *Accept:* `(Integer) "s"` rejected; `(List<String>) objectVar` allowed (unchecked warning);
  `(String) new Object()` allowed.
- **JAVA-TYP-009** `SHOULD` Constant expression evaluator (JLS 15.29) for narrowing and for constant
  folding of `static final` fields (stub initializer text, `01` 1.2).

---

## 3.3 Numeric promotion, operators, conditionals

- **Unary numeric promotion** (5.6.1): `byte/short/char → int`, unboxing first.
- **Binary numeric promotion** (5.6.2): if either operand is `double` → `double`; else `float`; else `long`;
  else `int`. Unboxing applies first. Used by `+ - * / %`, comparison, equality of numerics, `& | ^` on integral
  types, and the shifts use unary promotion on each side separately.
- `+` is string concatenation when either operand is `String` (the result is `String`).
- Compound assignment `a op= b` is `a = (T)(a op b)` with an implicit narrowing cast; increment/decrement keep
  the variable's type.
- Equality `==`/`!=`: numeric (after promotion) if both are numeric (one primitive suffices to trigger
  unboxing of the other), boolean if both boolean, reference otherwise where the operand types must be
  castable to each other; comparing `int` with a non-numeric reference is an error.
- Conditional `c ? a : b` (JLS 15.25): classify. *Boolean* if both operands are boolean/Boolean; *numeric* if
  both are numeric types (rules: same type; byte/short mix; constant `int` fitting the other narrower type
  gives the narrower; otherwise binary promotion); else *reference*: a **poly expression** in assignment/
  invocation context whose branches are each checked against the target type; as a standalone expression its
  type is `lub(boxed(a), boxed(b))` (3.7).
- Shifts, bitwise on `boolean`, `!` on boolean, `~` on integral, `instanceof` result `boolean`.
- Switch selector types: integral up to `int` and wrappers, `String`, enums, and any reference type with
  patterns (Java 21).

- **JAVA-TYP-010** `MUST` Promotion and result types of all operators per above (incl. `char + int = int`,
  `byte + byte = int`, `1 / 2.0 = double`, `"a" + 1 = String`, `Integer + Integer = int`).
- **JAVA-TYP-011** `MUST` Conditional expression typing: numeric/boolean/reference; the reference form as
  poly expression with a target; standalone type is lub of boxed branches.
  *Accept:* `true ? 1 : 2.0` is `double`; `true ? 1 : "a"` is `Object & Serializable & Comparable<...>`;
  `Object o = true ? 1 : "a"` is accepted; `byte b = true ? 1 : 2` accepted (constants).
- **JAVA-TYP-012** `MUST` Equality operand compatibility errors map to `04` 035 (`Operator '==' cannot be
  applied to ...`).

---

## 3.4 Generics: substitution, bounds, capture, erasure

- **Substitution**: replace type variables by type arguments in a type, recursively into arguments,
  arrays, wildcards bounds and intersections; unbound variables stay. A substitutor for member access on a
  parameterized receiver maps the *declaring class's* parameters (obtained through the supertype path, 3.2.1)
  and, for generic methods, the method's own parameters (from explicit type arguments or inference).
- **Raw types** (4.8): a use of a generic class without arguments. Members of a raw type are accessed with
  **erased signatures**: the member types are replaced by their erasure, except static members, and members
  inherited from non-generic supertypes (JLS 4.8). Calling a method whose erased parameter types differ from
  the declared ones is an *unchecked call* warning.
- **Bounds** (4.5): a type argument must be within the declared bounds of its type parameter after
  substitution (`class Box<T extends Number>`: `Box<String>` is an error `04` 045). F-bounded types
  (`T extends Comparable<T>`) are handled with a recursion guard.
- **Capture conversion** (5.1.10): applied to the type of an expression of parameterized type containing
  wildcards, *before* member access on it. For each wildcard argument a fresh captured variable is created:
  `? extends U` → upper bound `glb(U, bound of the parameter)`; `? super L` → lower bound `L`, upper bound
  the parameter bound; `?` → bound of the parameter. Capture is applied to the **top level** of a type
  (receiver of a call, result of a method call before it is used) and during inference when a wildcard-
  parameterized type is the argument of a subtyping constraint. Captured variables are not denotable and
  print as `capture of ?...`. Two captures of the same wildcard in different expressions are different types.
- **Erasure** (4.6): type variable → erasure of its leftmost bound; parameterized type → the raw class type;
  array → array of erasure; intersection → erasure of the first conjunct; primitives unchanged. Used for
  override-equivalence checks (`same erasure` errors, `04` 097), overload specificity of generic methods
  (JLS 8.4.2 subsignature), cast legality, and `instanceof` checks.
- **Intersection and bounds with several constraints**: `T extends A & B` has members of both; at most one
  bound may be a class and it must be first (error `04` 4.3).
- **Wildcard rules for expression types**: the type of a field/method access whose declared type mentions the
  class parameter is substituted after capture; `List<? extends Number>.get(0)` has type
  `capture of ? extends Number` which is assignable to `Number`; `add(1)` on it is an error (no argument is
  assignable to `capture of ? extends Number`).

- **JAVA-TYP-013** `MUST` Substitution through the supertype path (member types of inherited members of
  parameterized receivers). *Accept:* for `class A<T> { T get(){...} } class B extends A<String> {}`,
  `new B().get()` has type `String`.
- **JAVA-TYP-014** `MUST` Raw member access with erasure rules and unchecked warnings.
  *Accept:* `List raw; raw.add("x")` is an unchecked call; `raw.get(0)` has type `Object`.
- **JAVA-TYP-015** `MUST` Capture conversion at receivers and method results.
  *Accept:* `List<? extends Number> l; Number n = l.get(0);` ok; `l.add(1)` error (inference or
  applicability failure); `List<?> w; w.add(null)` ok.
- **JAVA-TYP-016** `MUST` Erasure function and its uses.
- **JAVA-TYP-017** `MUST` Bound checks on type arguments, with recursion guard for F-bounds.
- **JAVA-TYP-018** `SHOULD` `glb` for intersections of bounds.

---

## 3.5 Least upper bound (lub) and greatest lower bound

`lub(T1..Tn)` (JLS 4.10.4) is the intersection of the **minimal erased candidate** supertypes shared by all
Ti, each parameterized by the lub-of-arguments rule: for shared generic supertypes `G<A...>`, `G<B...>`
→ `G<lcta(A,B)>` where `lcta(U,V)` is `U` if equal, else `? extends lub(U,V)`; recursion on the same pair
produces `?` after a fixed depth. The result is an intersection if more than one candidate remains;
`Object` conjunct dropped when others exist. Primitive operands are boxed first.

**IJ**: cuts recursion by remembering the (type, type) pairs already being compared, and returns an
unbounded wildcard on re-entry; arrays: lub of component types; `null` is ignored; captured and
disjunction types are replaced by their upper bounds / lub; the result may be a (possibly unresolved)
class type when a component is unresolved.

- **JAVA-TYP-019** `MUST` lub per above. *Accept:* `lub(Integer, Double)` is `Number & Comparable<? extends
  Number & Comparable<?>>`; `lub(String, StringBuilder)` is `Object & Serializable & Comparable<...> &
  CharSequence` (presentable: `Object & Serializable & Comparable<? extends ...> & CharSequence`);
  `lub(ArrayList<String>, LinkedList<String>)` is `AbstractList<String> & List<String> & ...`.
- **JAVA-TYP-020** `SHOULD` Recursion cut-off with unbounded wildcard.

---

## 3.6 Boxing, varargs

- Boxing/unboxing as in 3.2.2 items 4-5; `Integer` wrapper cache semantics are not modeled.
- **Variable-arity methods** (8.4.1): the last formal `T... a` has type `T[]`. A call is applicable by
  variable-arity invocation (phase 3) with `n >= params-1` arguments, where the extra arguments are
  checked against `T` (loose invocation) and **a single argument of an array type assignable to `T[]` makes
  the call applicable by phases 1-2** (no wrapping). `null` as the single argument is ambiguous between
  `T[]` and `T`: the call compiles as passing `null` array (with a warning "non-varargs call of varargs
  method with inexact argument type" in javac); an inspection exists (`NullArgumentToVariableArgMethod`).
  A generic varargs element type that is not reifiable gives an *unchecked generic array creation*
  warning unless the method is annotated `@SafeVarargs`.
- Varargs with primitives: `foo(int... a)` and `foo(Object... a)` with argument `1`: phase 3 both
  applicable, `int...` more specific (JLS 15.12.2.5 with primitive preference): **IJ** picks the primitive
  varargs overload (a dedicated tie-break exists for primitive varargs).

- **JAVA-TYP-021** `MUST` Varargs applicability phases and array-passthrough, `null` argument case, in
  overload resolution (02 2.4).
- **JAVA-TYP-022** `SHOULD` `@SafeVarargs` and unchecked generic array creation warning.
- **JAVA-TYP-023** `MUST` Primitive-varargs tie-break.

---

## 3.7 Expression typing: summary table

| Expression | Type |
|---|---|
| literal | `int`, `long`, `float`, `double`, `char`, `boolean`, `String`, null type |
| local/field/parameter reference | declared type after substitution and **capture** (for non-`var`); `var` uses the initializer's type after upward projection (3.9) |
| method call | return type after inference/substitution and capture; for `getClass()` `Class<? extends |erasure of receiver|>` |
| `new C(...)` | `C` with arguments (or inferred for diamond); anonymous: the anonymous class type |
| array creation / initializer | `T[]` / target-typed |
| cast | the cast type (for intersection casts the intersection) |
| `a ? b : c` | 3.3 |
| lambda / method reference | no standalone type; takes its target functional interface type |
| `switch` expression | poly: each result expression against the target; standalone: lub of boxed result types |
| `this` / `super` | the enclosing class type (with its own type parameters as arguments) / its superclass |
| `a.length` | `int`; `arr[i]` component type |
| `instanceof`, comparisons, `!`, `&&`, `||` | `boolean` |
| assignment | type of the left side |

- **JAVA-TYP-024** `MUST` Typing of each expression kind in the table; `getClass()` special case.
- **JAVA-TYP-025** `MUST` Types of expressions are computed on demand and cached per tree revision; typing a
  method call triggers its overload resolution and inference (02 2.4, 3.8).

---

## 3.8 Inference (JLS 18) as IntelliJ applies it

IntelliJ implements the JLS 18 algorithm (an `InferenceSession` per generic call with a graph of inference
variables), including nested generic calls, lambdas and method references. The implementer must follow the
JLS; this section gives the structure and the places where observable behavior was pinned by IntelliJ's
implementation.

### 3.8.1 Objects

- **Inference variable** `α` per type parameter of the method being invoked (or class, for diamond), with a
  **bound set**: equality bounds `α = T`, upper bounds `α <: T`, lower bounds `T <: α`; the declared bound
  `P extends B` of the parameter seeds the upper bound `α <: B[P:=α]` (`Object` if none).
- **Constraint formulas** (JLS 18.2): `‹e → T›` (expression compatible with T in a loose invocation
  context), `‹S → T›`, `‹S <: T›`, `‹S <= T›` (containment), `‹S = T›`, `‹lambda → T›`, `‹mref → T›`.
- **Bound set operations**: *incorporation* (18.3): from `α = S` and `α = T` derive `S = T`; from `S <: α`
  and `α <: T` derive `S <: T`; substitute equalities; from `α = U` and bounds derive consequences; capture
  bounds (18.3.2) generate fresh variables for captured wildcards; false is derived when `S <: T` fails.
- **Resolution** (18.4): pick a set of variables whose dependencies are resolved; instantiate each:
  with a lower bound `lub(L1..Lk)` of proper lower bounds; else with proper upper bounds `glb(U1..Um)`; if the
  variable appears in a capture bound or instantiation fails with an error, use **fresh type variables**
  (JLS 18.4 second attempt). Dependencies: `α` depends on `β` if `β` appears in a bound of `α` (or the
  converse for capture); strongly connected groups are resolved together.

### 3.8.2 The invocation process (JLS 15.12.2.2-15.12.2.6, 18.5)

1. **Applicability inference (18.5.1).** For a candidate generic method with arguments `e1..en`, build the
   initial bound set `B0` from the type parameter bounds; add `‹ei → Fi[P:=α]›` for each argument *pertinent
   to applicability* (not an implicitly typed lambda, not an inexact method reference, not a poly
   conditional/switch whose pertinent status depends on variables); reduce; check the bound set is not
   false and, when resolution is needed to check, resolvable. The result is `B2`. Strict/loose/varargs
   phases (02 2.4.2) use different compatibility in `‹ei → Fi›` (strict: no boxing; varargs: element type
   for the extra args).
2. **Choosing the most specific** among applicable candidates (15.12.2.5, 18.5.4 for generic ones:
   "more specific method inference" tests whether the parameter types of `m1` can satisfy `m2`'s with
   fresh inference variables).
3. **Invocation type inference (18.5.2).** For the chosen method: start with `B2`; if the call is a poly
   expression (generic method whose return type mentions its type parameters, in an assignment or
   invocation context with a target type `T`): add `‹R θ → T›` with the necessary adjustments (unchecked
   conversion handling, boxing of an inference-variable return type when the target is primitive, wildcard
   return capturing); add constraints for the arguments *not* pertinent to applicability (lambdas, method
   references, poly arguments) in dependency order using their *input variables* (the lambda's parameter
   types) and *output variables* (return types); resolve the variables; the substitutor gives parameter and
   return types. Errors encountered here are **inference errors** (reported as `04` 047) and, when the target
   mismatches, as incompatible types with a `reason:` line.
4. **Nested generic calls.** An argument that is itself a poly method call is typed *within* the outer
   inference session: its type parameters join the bound set (the "return type is an inference variable"
   case), so that `foo(Collections.emptyList())` gives `emptyList` a target through `foo`'s parameter and
   `bar(List.of(1, 2))` types the inner call after the outer chose a compatible parameter.
5. **Diamond** `new C<>(args)` (15.9.3): treat as a generic method `<A1..An> C<A1..An> new(params)` of
   the class's constructors (the class type parameters become the inference variables), then as above.
   For an anonymous class with diamond (9+), the inferred type must be denotable (not captured or
   intersection) else the error `04` 050.
6. **Lambda parameters** (18.5.3, functional interface parameterization): an **explicitly typed** lambda
   with a wildcard-parameterized target infers the interface parameterization from its parameter types
   (JLS 18.5.3); an **implicitly typed** lambda takes the parameter types of the *function type* of the
   target after resolving the input variables; a lambda body's return expressions give constraints
   `‹ret → R›` where `R` is the function type's return.
7. **Method references** (`A::m`, 15.13): *exact* (one non-overloaded, non-varargs, non-generic member)
   participates in applicability; inexact ones are resolved after input variables. Constructor references
   `C::new` and array constructor `int[]::new` are handled the same. The search for the referenced method
   uses the function type's parameter types: first as `m(P1..Pn)` (static or unbound receiver variants
   decided by the qualifier kind), second `m(P2..Pn)` with `P1` as the receiver (unbound receiver form).
8. **Intersections and the target type**: when the target contains wildcards, the *ground target type* is
   derived (non-wildcard parameterization 9.9) before lambda typing.
9. **Return-type adjustments** (18.5.2.1): if `R θ` is an inference variable `α` and the target `T` is
   primitive, `α` is resolved first, and boxing/unboxing governs compatibility; if the method's return
   type is a parameterized type with captured wildcards, capture is applied to `R θ` before reduction.

### 3.8.3 Properties the implementation must reproduce

- Results are **stable and context-sensitive**: the same call in a different target context can infer
  different type arguments; the type of a method call expression used as an *argument* is computed in the
  context of the outer call, while the same expression typed in isolation uses the target-less rules
  (an `emptyList()` standalone is `List<Object>`).
- **Unchecked conversion during applicability or the return type**: when unchecked conversion was necessary
  for applicability, the return type is the *erasure* of the declared return type (JLS 15.12.2.6), and the
  call is flagged unchecked.
- **Incomplete code tolerance**: arguments whose type is unknown (unresolved symbols) do not fail the
  inference of the call but suppress inference-based errors (the call is "potentially compatible"),
  keeping a single root-cause error. **IJ** uses a distinguished lambda-parameter "unknown" type for this.
- **Inference failure explanation**: an inference error carries a one-line reason presented in the message
  (`04` 047), shaped as `inference variable T has incompatible bounds` + `equality constraints: ...` +
  `lower bounds: ...`, or `no instance(s) of type variable(s) T exist so that X conforms to Y`.
- **IJ registry-controlled javac compatibility**: choosing fresh variables only for captured wildcards, and
  allowing unchecked subtyping during incorporation, are both enabled by default to match javac.

### 3.8.4 Scenarios (expected inferred types)

| Code | Inferred |
|---|---|
| `List<String> l = new ArrayList<>();` | `ArrayList<String>` |
| `var m = new HashMap<String, List<Integer>>();` | `HashMap<String, List<Integer>>` |
| `Arrays.asList(1, 2.0)` | `List<Number & Comparable<? extends Number & Comparable<?> & Constable & ConstantDesc>>` (the intersection printed with the exact JDK supertypes present) |
| `List<String> e = Collections.emptyList();` | `T := String` |
| `foo(Collections.emptyList())` with `void foo(List<String> a)` | inner `T := String` (poly argument) |
| `Stream.of("a","bb").map(s -> s.length())` | `Stream<Integer>`; lambda parameter `s : String` |
| `list.stream().map(String::length).collect(Collectors.toList())` | `List<Integer>` |
| `Comparator.comparing(String::length)` | `Comparator<String>` (with target `Comparator<String>`; standalone: inference of `T` from the method reference's exact form gives `Comparator<String>`) |
| `Optional.of(1).map(i -> "s" + i).orElse("d")` | `String` |
| `Map<String, List<Integer>> m = ...; m.computeIfAbsent("k", k -> new ArrayList<>()).add(1);` | `computeIfAbsent` returns `List<Integer>`; the lambda's diamond infers `ArrayList<Integer>` |
| `Collectors.groupingBy(Person::city, Collectors.counting())` | `Collector<Person, ?, Map<String, Long>>` |

- **JAVA-TYP-026** `MUST` Bound-set based inference with incorporation and resolution (including fresh
  variables) for generic method calls: applicability inference, most-specific, invocation type inference
  with target typing.
- **JAVA-TYP-027** `MUST` Nested generic calls inferred in the context of the outer call.
- **JAVA-TYP-028** `MUST` Diamond inference for classes and anonymous classes.
- **JAVA-TYP-029** `MUST` Implicitly typed lambdas: parameters from the function type; return constraints.
  Explicit lambdas: parameterization inference.
- **JAVA-TYP-030** `MUST` Method references: exact/inexact, the two-search form, constructor/array
  references.
- **JAVA-TYP-031** `MUST` Unchecked-conversion adjustment of the return type and flagging.
- **JAVA-TYP-032** `MUST` Incomplete-code tolerance (no cascade of inference errors).
- **JAVA-TYP-033** `SHOULD` Inference error messages with reasons as in `04` 047.
- **JAVA-TYP-034** `MUST` Standalone vs target-typed results as in 3.8.3 (first bullet).
- **JAVA-TYP-035** `SHOULD` Conformance corpus: the scenario table plus the JDK stream/collector idioms
  (`Collectors.*`, `Comparator.*`, `Optional`, `CompletableFuture` chains); differential tests against
  javac/jdtls report any divergence.
- **JAVA-TYP-036** `LATER` Performance guard: bound-set work limits and cancellation points so a pathological
  nested generic call cannot hang the editor.

---

## 3.9 `var`, upward projection, switch and patterns

- `var x = e;` has the type of `e` after **upward projection** (JLS 4.10.5): captured wildcards and
  intersection components that are not denotable are replaced by their bounds so that the variable type is
  denotable (`var l = new ArrayList<? extends Number>()` is illegal; `var x = list.get(0)` with
  `List<? extends Number>` is `Number`). Not allowed: no initializer, `null`, lambda, method reference, array
  initializer, self reference (errors `04` 055).
- Type patterns `o instanceof T t` require `T` castable from the operand type and not a supertype of it (error
  if redundant before Java 21); record patterns infer the type arguments of the record type from the operand
  type (JLS 18.5.5) and recursively for nested patterns.
- Switch exhaustiveness uses sealed hierarchies and record-pattern coverage (JLS 14.11.1.1) over the
  permitted subclass sets.

- **JAVA-TYP-037** `MUST` `var` typing with upward projection and its restrictions.
- **JAVA-TYP-038** `SHOULD` Record pattern type inference and nested coverage.
- **JAVA-TYP-039** `SHOULD` Sealed/exhaustiveness queries.

---

## 3.10 Functional interfaces

A type `F` is a functional interface if it is an interface with exactly one abstract method that is not
an `Object` public method (override-equivalent members count once; `default` and `static` methods do not
count; generic abstract methods are allowed for method references but not for lambdas: `04` 4.7 `lambda.sam.generic`).
The **function type** of a parameterized `F` is the abstract method's signature after substitution
(parameter types, return type, thrown types). For wildcard-parameterized `F` use the non-wildcard
parameterization (9.9). `@FunctionalInterface` is checked against this definition.

- **JAVA-TYP-040** `MUST` Functional interface detection and function type (with substitution).
- **JAVA-TYP-041** `SHOULD` Wildcard parameterization derivation (9.9).

---

## 3.11 Open questions

- **Q-TYP-1** IntelliJ type-use nullability (a nullability dimension on types for JSpecify) is a newer feature;
  scope it with `07-inspections.md` nullability decisions before adding to the type model.
- **Q-TYP-2** Exact printing of lub intersections with the JDK's `Constable`/`ConstantDesc`: depends on the JDK
  version; tests should pin the JDK.

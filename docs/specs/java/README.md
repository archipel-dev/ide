# Java language intelligence: specification

Audience: the implementation team of Archipel IDE (Phase 4 of `docs/architecture/platform-refactor-plan.md`:
"Java/Kotlin resolve on the project model"). Goal: Java analysis exactly as good as IntelliJ IDEA's.

Process: clean-room. These documents were written from the IntelliJ IDEA Community sources and the Java
Language Specification, **in our own words**. They contain no JetBrains code, no copied comments and no
method-by-method structure; message texts quoted from message bundles are short user-visible facts. The
implementation team builds only from these documents. Citations are `ic: <path>` relative to the root of the
Community repository (tag as in `docs/analysis/code-intelligence-spec.md`) so a reviewer can audit a claim.

## Files

| File | Topic | ID prefix |
|---|---|---|
| [01-psi-and-stubs.md](01-psi-and-stubs.md) | What each declaration stores in stubs; stub index keys; what source stubs skip; synthetic members; dumb mode | `JAVA-STUB` |
| [02-resolve.md](02-resolve.md) | Scope walk order; shadowing and obscuring; overload resolution; file-level scope (imports, package, `java.lang`, modules); qualified names; resolve scope (test vs production); class-file stubs, source attachment, JDK | `JAVA-RES` |
| [03-types.md](03-types.md) | Type model, subtyping, conversions, generics, capture, erasure, lub, boxing, varargs, inference (JLS 18) as IntelliJ applies it, diamond, lambdas, `var` | `JAVA-TYP` |
| [04-error-catalog.md](04-error-catalog.md) | Every error the Java highlighter reports: exact message, range, trigger, fixes (about 140 detailed rows) plus the full message index (Appendix A, 466 keys) | `JAVA-ERR` |
| [05-imports.md](05-imports.md) | The reported `assertThat` bug end to end; static-import fix; class-import fix; hint; auto-import; unused imports; Optimize Imports; settings | `JAVA-IMP` |
| [06-completion.md](06-completion.md) | Contributors and order; positions; library classes and import on completion; static members; relevance; chain completion (LATER) | `JAVA-CMP` |
| [07-inspections.md](07-inspections.md) | 40+ default-on inspections with message, severity, range, trigger, fixes | `JAVA-INS` |
| [08-spring.md](08-spring.md) | What Community knows about Spring (almost nothing) and what a Spring pack would need | `JAVA-SPR` |

Suggested reading order for a first implementation slice that fixes the bug: 05 (sections 5.1-5.4), 02
(2.5, 2.7, 2.8), 01 (1.2, 1.3), 04 (4.1, 4.2 rows 007-012), then 03 (3.2) for applicability, then the rest.

## Conventions

- **Requirement IDs** `JAVA-<AREA>-<NNN>`: stable; never reused. A letter suffix (`019a`) is an addendum.
  Each requirement has a one-line acceptance criterion (`Accept:`) or is a table row whose columns define it.
  Error-catalog rows are requirements by themselves.
- **Priority**: `MUST` (needed for the Phase 4 gate "the import scenarios pass without jdtls and the
  differential test shows no regressions"), `SHOULD` (visible quality gap against IntelliJ), `LATER`
  (valid behavior, schedule after the gate).
- **IJ** marks an observable IntelliJ choice that is not in the JLS; the JLS is the semantic authority
  otherwise.
- **Deliberate deviations** are called out where IntelliJ's behavior is unspecified (for example tie order of
  static-import candidates, 05 section 5.3): the Archipel rule is deterministic.
- Scenarios are small Java snippets written for this spec (inputs, expected markup/message/Alt+Enter items,
  resulting file). They are meant to be turned into the conformance corpus (Phase 0).

## Counts (requirements with a priority)

| File | MUST | SHOULD | LATER | Total |
|---|---|---|---|---|
| 01 PSI and stubs | 19 | 7 | 2 | 28 |
| 02 Resolve | 30 | 15 | 2 | 47 |
| 03 Types | 30 | 10 | 1 | 41 |
| 04 Error catalog (detailed rows and rules; Appendix A adds 466 index rows as one `MUST`+one `LATER` bulk requirement) | 59 | 81 | 12 | 152 |
| 05 Imports | 39 | 22 | 5 | 66 |
| 06 Completion | 16 | 17 | 4 | 37 |
| 07 Inspections | 34 | 27 | 1 | 62 |
| 08 Spring | 0 | 2 | 3 (+7 unnumbered feature bullets, all `LATER`) | 5 |
| **Total** | **227** | **181** | **30** | **438** |

## The reported bug in one paragraph

`assertThat(x)` in a JUnit test with AssertJ on the test classpath and no import: IntelliJ reports
`Cannot resolve method 'assertThat(String)'` on the identifier `assertThat` only. Alt+Enter's first item is
`Import static method...`, which searches the method-name index in the *resolve scope of the file* (test
libraries are visible only to test files), keeps static, accessible members, **keeps one member per class and
only classes where the call's arguments are applicable** (so Hamcrest/JUnit `assertThat` with two/three
parameters are not offered for a one-argument call), sorts by deprecation, nesting, learned usage and a
proximity chain (ties are unspecified in IntelliJ; Archipel sorts by class name so `Assertions` is first), shows a
`Method to Import` popup when there are several, and applies `import static
org.assertj.core.api.Assertions.assertThat;` in the static group (collapsing to `.*` at the third name from the
same class). Details and scenarios: [05-imports.md](05-imports.md).

## Cross-document dependencies

- 05 relies on 01 (method-name and class-name indexes), 02 (resolve scope 2.7, accessibility 2.9) and 03
  (applicability: 3.2, 3.8).
- 04's wrong-argument and unresolved-call errors are selected by the overload outcome of 02 2.4.
- 06's class and static-member suggestions use the same indexes, scope and import machinery as 05.
- 07's unused-symbol rules use the reference tables of 02; its nullability rows depend on a data-flow engine
  that is not specified here (07 Q-INS-1).

## Open questions (collected)

1. Exact title of the create-method quick fix (05 Q-IMP-1).
2. Whether IntelliJ's strict/loose phase handling inside most-specific selection differs observably from the
   JLS phase model (02 Q-RES-1, Q-RES-2); resolve by differential tests.
3. Data-flow engine specification (07 Q-INS-1).
4. Persistent format for usage statistics shared by import ordering and completion (06 Q-CMP-2).
5. Whether Spring is part of the base pack (08 Q-SPR-1).
6. Nullability on types (JSpecify) in the type model (03 Q-TYP-1).
7. Not covered (out of this task's list): Kotlin, Groovy, Javadoc rendering, formatter, refactorings, run/debug.

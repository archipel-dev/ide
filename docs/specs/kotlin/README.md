# Kotlin language intelligence: specification

Audience: the implementation team of Archipel IDE (own Kotlin analysis engine, no kotlin-lsp long term). Goal: what the IntelliJ Kotlin plugin
(K2 mode) does, specified on top of the Kotlin language specification (<https://kotlinlang.org/spec/>), which is cited, not restated.

Process: clean-room, as `docs/specs/README.md`. Sources: IntelliJ IDEA Community (`ic:` citations) and, for compiler diagnostic texts and ranges, the Apache-2.0
Kotlin compiler 2.4 sources (`FirErrors`, `FirErrorsDefaultMessages`, `PositioningStrategies`). Message texts are short user-visible facts.

## Files

| File | Topic | ID prefix |
|---|---|---|
| [01-psi-stubs-and-indexes.md](01-psi-stubs-and-indexes.md) | stubs, 20 indexes, receiver keys, file facades, dumb mode | `KT-STUB` |
| [02-resolve-and-imports.md](02-resolve-and-imports.md) | file scopes, default imports, Import fix (candidates, text, order, edit), Optimize Imports, layout | `KT-RES`, `KT-IMP` |
| [03-diagnostics-catalog.md](03-diagnostics-catalog.md) | compiler diagnostics vs inspections, ranges, about 110 rows with messages and fixes, suppression | `KT-DIAG` |
| [04-completion.md](04-completion.md) | positions, contributors, presentation, insertion, keywords, ordering, char filter, smart completion | `KT-CMP` |
| [05-inspections.md](05-inspections.md) | 87 detailed inspections with cases, naming, unused symbols, appendix of all 325 | `KT-INS` |
| [06-intentions-and-quickfixes.md](06-intentions-and-quickfixes.md) | 41 intentions with cases, create from usage, generate and other actions, appendix of all 176 intentions | `KT-INT`, `KT-QF` |
| [07-java-interop.md](07-java-interop.md) | light-class shape, navigation, find usages, platform types | `KT-JVM` |

Cases: `cases/NN-*.cases.md` (355 cases). Convention: `[prefix]` after a message = only the prefix is specified; `[verify]` = not confirmed against a running IDE.

## Counts (requirement rows with a priority)

| File | MUST | SHOULD | LATER | Total |
|---|---|---|---|---|
| 01-psi-stubs-and-indexes.md | 13 | 4 | 1 | 18 |
| 02-resolve-and-imports.md | 21 | 2 | 0 | 23 |
| 03-diagnostics-catalog.md | 50 | 43 | 1 | 94 |
| 04-completion.md | 11 | 7 | 1 | 19 |
| 05-inspections.md | 48 | 46 | 1 | 95 |
| 06-intentions-and-quickfixes.md | 43 | 7 | 6 | 56 |
| 07-java-interop.md | 5 | 2 | 0 | 7 |
| **Total** | **191** | **111** | **10** | **312** |

## Inventory coverage

Inspections: 325 rows in Appendix A of 05 (42 MUST, 192 SHOULD, 91 LATER with reasons). Intentions: 176 in Appendix A of 06. Actions (45), settings pages (3), run
configuration types (2): section 6.5 (MUST/SHOULD/LATER with reasons). Index, import and completion settings: 01, 02, 04.

## Open questions (collected)
1. Exact construct-keyword insertions and smart-completion catalogue (04 Q-CMP-1, Q-CMP-3).
2. Ranges of type-mismatch diagnostics when the whole declaration is the source (03 Q-DIAG-2); full diagnostics index (Q-DIAG-1).
3. Nested-class import edit and explicit imports of default-imported names (02 Q-IMP-1, Q-IMP-4).
4. Many intention row texts and defaults marked `[verify]` in 05/06 cases.
5. Light-class details for value classes and internal members (07 Q-JVM-1).

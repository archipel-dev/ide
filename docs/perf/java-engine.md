# Native Java engine: performance

Budgets of `docs/architecture/java-engine-plan.md` section 3.5, measured by
`cargo bench -p archipel-ide-java --bench completion` (release; `ARCHIPEL_PERF_ASSERT=1` exits non-zero when a
budget is exceeded). Fixture: the JDK 21 of the machine (exported packages), our hermetic JUnit-like jar and
generated sources of 180,000 classes in 600 files, so the name table of the test file has 187,970 classes
(the plan budgets 300k names). 4 cores, debug info off.

| Operation | Budget | Measured |
|---|---|---|
| Cold index: JDK image | < 8 s | 0.85 s |
| Cold index: JDK + jar + 180k generated classes | < 20 s | 3.5 s |
| Warm start (persisted, unchanged) to Ready | < 500 ms | 50 ms |
| Completion at a caret, `@Te` and `new ArrayLi` (class names), p95 / p99 | < 50 ms / < 100 ms | 7.5 ms / 13.9 ms |
| Completion, type position over 180k generated names, p95 / p99 | < 50 ms / < 100 ms | 17.7 ms / 23.0 ms |
| Import edit of the chosen class, p95 | < 50 ms | 0.8 ms |
| Add one jar | < 1 s | 15 ms |
| 100 changed source files | < 2 s | 0.77 s |
| Class-name query over the table (worst of ten patterns, ~100k matches), p50 / p95 | < 10 ms (300k names) | **~10-12 ms / 20-22 ms (188k names), over budget** (46 ms p95 on a loaded run) |
| Member access (`names.`, `stream().`, `get(0).`, JFrame with 1000+ members), p50 / p95 | < 50 ms | 5-9 ms / 6-20 ms |
| Variants (unqualified identifier) and keywords, p50 / p95 | < 50 ms | ~7 ms / 8-19 ms |
| Expected type `List<String> l = new Arr`, p50 / p95 | < 50 ms | 20 ms / 40 ms |
| Statics on the second press (`ma`), p50 / p95 | < 50 ms | 25 ms / 42-50 ms (at the budget) |

The machine is a shared VM: the same bench varies by a factor of two between runs (a loaded run showed the
class-name query at 46 ms p95, type position at 59 ms and statics at 50 ms). Numbers above are the quiet runs.

What was done for the class-name query (13.2 ms before): a first-letter scan (names are bucketed by first ASCII
letter, so a prefix pattern touches one bucket), a hump-letter mask per name (`NameEntry.humps`) that rejects
camel-hump patterns without running the matcher, and a bounded key set (limit * 4) so a pattern with 100k
matches does not rank them all. It is still over the 10 ms p95 budget for the worst patterns (one or two
letters matching most of the 188k generated names); an FST of the names (plan risk 6) is the next step and is
not done. Member completion became fast through an incremental reparse of the probe tree (previous tree per
path, the placeholder edit only; a full parse was ~25 ms) and a parent map built only around the focused
member. Cold index of the JDK 1.1-1.4 s, JDK + jar + 180k classes 3.8-5.9 s, warm start 0.06-0.1 s,
100 changed files 0.97-1.1 s, all within budget.
Index time of the JDK is low because the image reader skips compressed resources (counted, plan risk 7).

## Native error pass while typing (M2)

`cargo bench -p archipel-ide-java --bench errors` (release, `ARCHIPEL_PERF_ASSERT=1` exits non-zero over
budget). Fixture: a generated, error-free file of 2011 lines / 77 kB (generics, streams, lambdas, inner
classes, static imports, library receivers), the JDK 21 of the machine, warm index. One sample is the work of
one `java-resolve` pass after an edit: parse, repair of a broken buffer, stub, resolve, import usage. The
daemon adds its 100 ms Java debounce (`JAVA_DEBOUNCE`) before the pass starts.

| Operation | Budget | Measured (p50 / p95) |
|---|---|---|
| Pass over the clean 2000-line file | pass p95 < 150 ms | 79 ms / 106 ms |
| Pass after typing `foo()` without `;` in the middle | pass p95 < 150 ms | 93 ms / 127 ms |
| Pass after typing `Bar b` without `;` in the middle | pass p95 < 150 ms | 92 ms / 104 ms |
| Editor flow, `foo();` typed in a small file: last keystroke to the red `foo` in Problems | < 150 ms + debounce | 111 ms (`an_unknown_method_call_is_red_shortly_after_the_last_keystroke`) |

What made it fit (first measurement 207 ms p50 / 262 ms p95; parse alone is 13 ms, stub and analysis 67 ms): a parent table of the tree built once per
analysis (`Node::parent` walks down from the root on every call and the scope walk calls it per reference),
the answers of the index memoized per analysis (`Env::find_class`, `Env::package_exists`), the type of an
expression memoized per node (a call chain asked for its receiver at every link), and the import usage reusing
the parse of the pass. The analysis is a pure function of (text, index): per-member incremental re-analysis
and a result cache keyed by (revision, index epoch) are not needed for the budget and are not built.

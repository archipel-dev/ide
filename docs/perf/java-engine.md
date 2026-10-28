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
| Class-name query over the table (worst of ten patterns, 100k matches), p95 | < 10 ms (300k names) | **13.2 ms (188k names), over budget** (p50 7.4 ms) |

The class-name query scans every name of the scope with the matcher (multi-threaded above 50k names).
Prefix patterns that match half of the table are the worst case; a first-letter bucket or an FST of the
names (plan risk 6) is the next step and is not done. Everything else is within budget.
Index time of the JDK is low because the image reader skips compressed resources (counted, plan risk 7).

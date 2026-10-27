# Java native resolve errors versus jdtls

Differential check of the native `java-resolve` pass against jdtls on the same project
(`crates/archipel-ide-core/tests/java_differential.rs`, ignored; needs jdtls from the toolchain store,
Gradle and warm caches):

```bash
cargo test -p archipel-ide-core --test java_differential -- --ignored --nocapture
```

The project is a Gradle module (JUnit 5, AssertJ) with valid files that exercise generics, static
and on-demand imports, inner and local classes, lambdas and inheritance, plus `Broken.java` with
unresolved imports, types, fields, methods, a qualified type and an unknown receiver. Only
resolve-kind errors count (`cannot be resolved`, `is undefined for the type`, `Cannot resolve ...`).
Two errors agree when they are on one line and one range contains the other: jdtls underlines a whole
qualified name (`java.util.Lisst`) where native marks the first unknown segment (`Lisst`).

## Result

| | count |
|---|---|
| native resolve errors | 17 |
| jdtls resolve errors | 17 |
| native-only (false positives) | 0 |
| jdtls-only (misses) | 0 |

An earlier run found one miss: `this.nothing()` was not checked because `this` receivers were skipped.
It now goes through the normal qualified-call check (`super` is still skipped).

## Corpus false-positive hunt

`crates/archipel-ide-java/tests/corpus.rs` (ignored) analyzes a sources jar against a classpath.

| corpus | files | errors | remaining errors |
|---|---|---|---|
| assertj-core 3.26 | 789 | only missing libs | |
| asm, jupiter-api, jupiter-engine, logback-core | 35, 144, 93, 421 | only missing libs | |
| junit-platform-engine 1.11.0 | 91 | 0 | |
| spring-boot-test 3.3.4 | 83 | 31 | all for libraries absent from the classpath (Apache HttpClient, HtmlUnit, JUnit 4, GraphQL, Gson, JSON-B, Selenium, reactive client) |

## Decision

The native errors are clean against jdtls on the differential project and on the corpora, so the pass
is on by default (`java.native.analysis` = true). The setting stays so it can be switched off
(Settings | Editor | Inspections | Native Java analysis) if a project hits a false positive. The scope
is narrow (unresolved names only, nothing reported while a supertype, import or generated member could
supply the name), so a disagreement is rare; widening it to type errors needs a new run of this check.

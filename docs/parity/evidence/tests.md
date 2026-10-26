# Test runner tree

Captured 2026-10-10 under Xvfb (1600x1000, `ARCHIPEL_RENDERER=glow`), Gradle 8.14.3,
JDK 21 and Rust 1.97 on PATH. Reference: `reference/intellij-community/java-gradle/18-test-results.png`
(PR #40).

Samples:
- `java-gradle` with JUnit 5 (`useJUnitPlatform()`), `AppTest.greets()` passing and
  `AppTest.greetsLoudly()` failing. Maven Central returned HTTP 429 through the sandbox
  proxy, so the sample adds the Google mirror of Maven Central as a repository.
- `rust-calc`: a library with `tests::adds` passing, `tests::subtracts` failing and an
  `#[ignore]`d `tests::benchmark`.

| File | Steps → result |
|---|---|
| `tests/18-test-results.png` | Run widget → Run All Tests (`gradle test`): "Test" window with **Test Results → AppTest → greetsLoudly() ✗ 33 ms, greets() ✓ 1 ms**, header "1 test failed, 1 passed · 2 tests total, 34 ms" with a red bar; results read from `build/test-results/test/TEST-*.xml` |
| `tests/18b-test-failure-details.png` | clicking `greetsLoudly()` shows the assertion message and stack trace instead of the console |
| `tests/18c-rerun-failed-tests.png` | red rerun button (Rerun Failed Tests) → `gradle test --console=plain --tests com.example.AppTest.greetsLoudly`: "1 test failed · 1 test total" |
| `tests/18d-cargo-test-results.png` | Rust: `cargo test` parsed live from libtest output: benchmark (ignored), adds ✓, subtracts ✗ with the panic message and `left`/`right` details |

Differences from the reference: results appear after the Gradle test task finishes, not
streamed per test (no Tooling API events). There are no per-test durations for libtest.
The toolbar is reduced to Rerun, Rerun Failed and Stop plus a "Hide passed" checkbox, with
no sort, export, history or auto-test toggles. Clicking a test does not navigate to its
source yet, and there are no gutter test icons.

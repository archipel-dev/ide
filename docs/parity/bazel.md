# Bazel parity

Reference: IntelliJ IDEA Community 2025.2 with the Bazel plugin (issue #45). Evidence:
`evidence/bazel.md`.

| ID | Reference | Feature | Status | Weight | Commands | Tests | Limitations / evidence |
|---|---|---|---|---:|---|---|---|
| BAZEL-001 | IntelliJ Bazel plugin | Detect Bazel workspace | Implemented | 5 | `build.detect` | workspace detection tests | `MODULE.bazel`, `WORKSPACE(.bazel)`, `.bazelversion`; `bazelisk` preferred over `bazel`, actionable error when neither is on PATH |
| BAZEL-002 | IntelliJ Bazel plugin | Sync (target import) | Partial | 5 | `bazel.sync`, `bazel.targets` | label_kind parser tests, fake-bazel command test, real Bazel 7.4.1 walkthrough | `bazel query 'kind(rule, //...)'` → targets cached in `.archipel/bazel/targets.json`; no source-root/classpath mapping for the Java language server yet |
| BAZEL-003 | IntelliJ Bazel plugin | Targets tool window | Partial | 3 | `toolwindow.activate.bazel`, `bazel.runTarget` | row grouping/filter tests | right stripe; packages → targets with kind, filter, Build/Run/Test toolbar, double-click runs the rule's default action; no context menu or "jump to BUILD file" |
| BAZEL-004 | IntelliJ Bazel plugin | Build / run / test a target | Implemented | 5 | `bazel.runTarget`, `runConfig.run` | fake-bazel command test, configuration plan tests | creates a temporary Bazel run configuration (`run //:hello`) that can be saved; `bazel build //...` for Build Project |
| BAZEL-005 | IntelliJ Bazel plugin | Test results in the test tree | Partial | 3 | `test.project`, `test.rerunFailed` | test.xml label mapping, summary-line (cached) and rerun tests | `bazel-testlogs/**/test.xml` plus console summary lines (cached results); Rerun Failed re-tests failed targets; per-test-case results inside a target only when the test framework writes them |
| BAZEL-006 | IntelliJ Bazel plugin | Gutter run icons in BUILD files | Partial | 3 | `run.gutter` | rule line scanner test, gutter mapping via the synced target list | one icon per synced rule with Run/Test/Build by rule kind; needs a Bazel sync first; `evidence/gutter/x03e-gutter-bazel-build-file.png` |
| BAZEL-007 | IntelliJ Bazel plugin | Java sources mapped to targets | Missing | 5 | — | — | needed for completion/navigation in Bazel Java projects |

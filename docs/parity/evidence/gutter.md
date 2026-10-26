# Gutter run icons

Captured 2026-10-10 under Xvfb (1600x1000, `ARCHIPEL_RENDERER=glow`), Gradle 8.14.3, JDK 21,
Rust 1.97 and Bazelisk/Bazel 7.4.1 on PATH. Reference:
`reference/intellij-community/java-gradle/x03-gutter-run-popup.png` (PR #40).

Parity rows: IJ-RUN-011, IJ-RUN-060, IJ-RUN-002 and IJ-BLD-007 in
`docs/parity/intellij-run-debug.md` / `intellij-build-tools.md`, and BAZEL-006 in
`docs/parity/bazel.md`.

| File | Steps → result |
|---|---|
| `gutter/x03-gutter-run-popup.png` | `App.java`: ▶ next to line 13 (`public static void main`); clicking it opens **Run 'App.main()'** (Ctrl+Shift+F10), Debug 'App.main()' (disabled, no debugger yet) and **Modify Run Configuration…**, as in the reference |
| `gutter/x03b-gutter-run-main.png` | Run → temporary configuration "App" runs `gradle :archipelRunMain --init-script … -Parchipel.mainClass=com.example.App` (a JavaExec task from Archipel IDE's init script, so any main class runs through Gradle); "Hello, Gradle!" |
| `gutter/x03c-gutter-run-test.png` | `AppTest.java`: class icon on line 7 and method icons on lines 9 and 14; running `greetsLoudly` → `gradle :test --tests com.example.AppTest.greetsLoudly` → test tree with the single failure |
| `gutter/x03d-gutter-rust-tests.png` | `lib.rs` (Cargo): module icon on `mod tests`, method icons on `adds`, `subtracts`, `benchmark` (→ `cargo test -- tests::adds --exact`) |
| `gutter/x03e-gutter-bazel-build-file.png` | `BUILD.bazel`: one icon per synced rule; the popup offers **Run '//:hello'** |

Detection uses the tree-sitter grammars of the language registry (`archipel-ide-runnables`):
Java/Kotlin `main` and `@Test`/`@ParameterizedTest`/… methods and their classes, Rust `fn main`
in binaries and `#[test]`/`#[tokio::test]` functions with their module path, Go `func main`
and `TestXxx`/`BenchmarkXxx`. Mapping: Gradle (JavaExec / `--tests`), Maven (`-Dtest`), Cargo
(`--bin`/`--test`/`--example`, `--exact`), Go (`go test -run`), Bazel (rule label).

Differences from the reference: the icon has no pass/fail state from the last run yet, there
is no "Run with Coverage", Debug is disabled until the debugger slice, and there are no icons
in the Project tree or on test directories.

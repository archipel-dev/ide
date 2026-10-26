# Bazel targets, build/run/test

Captured 2026-10-10 under Xvfb (1600x1000, `ARCHIPEL_RENDERER=glow`) with **Bazelisk 1.25.0**
on PATH (downloaded Bazel 7.4.1 via `.bazelversion`). Sample `bazel-java` (Bzlmod
`MODULE.bazel`): `//:app` (`java_binary` → `//lib:greeter` `java_library`), `//:hello`
(`sh_binary`), `//:app_test` (`sh_test`, passes), `//:flaky_test` (`sh_test`, fails).
The sandbox proxy answers 403 for the `rules_proto` archive that Bazel's Java toolchain
fetches, so Java targets cannot build here. That is an environment limit and shows as a
Bazel error in the console. Shell rules build, run and test normally.

There is no reference screenshot of the IntelliJ Bazel plugin's target window in PR #40
(only import/new-project/plugin scenes b05–b07); the layout follows the Gradle tool window
(scene 21) and the Run/Test windows (scenes 17/18).

| File | Steps → result |
|---|---|
| `bazel/21a-bazel-not-synced.png` | open the workspace, click the cube on the right stripe → "Bazel project is not synced yet" with Sync; Run is disabled and its tooltip points to the Bazel tool window |
| `bazel/21-bazel-targets-synced.png` | Sync → `bazelisk query "kind(rule, //...)" --output=label_kind --keep_going` in the Build window ("sync finished 316 ms"); tree `//` → app (java_binary), app_test, flaky_test (sh_test), hello (sh_binary); `//lib` → greeter (java_library); generated `*_internal_rule` targets hidden |
| `bazel/17-bazel-run-target.png` | select `hello` → toolbar Run → `bazelisk run //:hello` prints "Hello from Bazel run"; the run widget now shows the temporary configuration "run //:hello" |
| `bazel/18-bazel-test-target.png` | double-click `flaky_test` → `bazelisk test //:flaky_test`: test tree `//:flaky_test → flaky_test` failed (from `bazel-testlogs/flaky_test/test.xml`) |
| `bazel/18b-bazel-test-all.png` | Run All Tests → `bazelisk test --keep_going //...`: `//:app_test` passed (cached; read from Bazel's summary line), `//:flaky_test` failed |

Bug 9 of #47 (Run/Build looked enabled and only failed in the status bar) was fixed in #60;
with Bazel installed, Build now runs `bazel build //...` in the Build window.

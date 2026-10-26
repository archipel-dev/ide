# Build / Run tool window and build-system detection

Captured 2026-10-10 on Linux under Xvfb (1600x1000, `ARCHIPEL_RENDERER=glow`,
`ARCHIPEL_NO_UPDATE_CHECK=1`, debug build), driven with `xdotool`
(`Ctrl+F9` Build, `Shift+F10` Run). Gradle 8.14.3 and OpenJDK 21 were on PATH; Bazel
was deliberately **not** installed. Reference scenes:
`reference/intellij-community/java-gradle/16-build-output.png` and `17-run-output.png`
(PR #40).

Sample projects (recreate with the commands in the PR description):

- `java-gradle`: single-module `application` project (no wrapper, so `gradle` from PATH).
- `MyApplication`: Android layout (`settings.gradle.kts` includes `:app`, whose build
  script applies `alias(libs.plugins.android.application)`) with a fixture `gradlew` that
  only echoes its arguments.
- `bazel-java`: `MODULE.bazel` + `BUILD.bazel`.

| File | Scene | Shows |
|---|---|---|
| `build-run/16-build-output.png` | 16 | `gradle build --console=plain`: tree root "java-gradle: build finished 1 sec, 663 ms", one node per Gradle task with its outcome (UP-TO-DATE, NO-SOURCE), console on the right with the echoed command and the coloured `BUILD SUCCESSFUL` line |
| `build-run/16b-build-output-failed.png` | 16b | compile error: root "build failed (exit code 1)", `:compileJava FAILED` with the javac error `cannot find symbol App.java:14` and Gradle's failure reason; clicking the error opened `App.java` at 14:1 (status bar), duplicates from Gradle's failure summary are merged |
| `build-run/17-run-output.png` | 17 | `gradle :run`: "Run" window, root "java-gradle [Gradle Run]: successful 1 sec, 575 ms", program output in the console |
| `build-run/android-17-run-disabled.png` | bug 8 | Android project: the run widget shows "Gradle Assemble", ▶ is disabled and its tooltip explains that `:app` is an Android application and needs an Android run configuration. `Shift+F10` does nothing; `./gradlew run` is never started |
| `build-run/android-16-build-assemble.png` | bug 8 | Build on the Android project runs `./gradlew assembleDebug --console=plain` |
| `build-run/bazel-16-build-missing-tool.png` | bug 9 | Bazel workspace is detected ("Bazel" widget; Run disabled with "create a run configuration with a target such as //:app"). Build without Bazel installed opens the Build window with "could not start" and the full actionable message (install Bazelisk or Bazel) instead of only a truncated status-bar text |

Bug 12 (first output line clipped, blank runs from progress output): the console is now a
separate pane below the tool-window header with no overlaid status line; output is split
into complete lines (`ConsoleBuffer`), `\r` progress redraws and ANSI sequences are
removed, and runs of blank lines collapse to one. Covered by `console::tests`.

## Remaining differences from the reference

- The tool window has no tabs ("Sync", "Build Output", multiple run tabs); one task at a
  time, title switches between Build/Run/Test.
- Gradle task timings are measured by Archipel IDE from console arrival (poll granularity), not
  from Gradle Tooling API events, and there is no per-task console filter.
- No Gradle sync, Gradle tool window or run configurations yet (next slices).
- No "Run" stripe button separate from Build; the run widget popup offers Run, Build
  Project, Run All Tests and Rerun Last Task.
- The rerun icon is an original drawing, not the platform icon.

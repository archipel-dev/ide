# Gradle sync and Gradle tool window

Parity rows: IJ-BLD-002 and IJ-BLD-004 in `docs/parity/intellij-build-tools.md`.

Captured 2026-10-10 on Linux under Xvfb (1600x1000, `ARCHIPEL_RENDERER=glow`,
`ARCHIPEL_NO_UPDATE_CHECK=1`), Gradle 8.14.3 on PATH, OpenJDK 21. Project: the
single-module `java-gradle` sample from `build-run.md` (`application` plugin, no wrapper).
Reference: `reference/intellij-community/java-gradle/21-gradle-tool-window.png` and
`reference/android-studio/a06-gradle-sync.png` (PR #40).

| File | Scene | Steps → result |
|---|---|---|
| `gradle/21a-gradle-not-synced.png` | 21 (before sync) | Open project, click the cube button on the right stripe → "Gradle" window with toolbar (Sync, Execute Gradle Task, Expand All, Collapse All) and "Gradle project is not synced yet" with a **Sync All Gradle Projects** button. Nothing ran on open |
| `gradle/a06-gradle-sync.png` | a06 | Click Sync → Build window titled "Sync": "java-gradle: sync finished 2 sec, 507 ms", the `:archipelModel` step, console with the init-script invocation; status bar "Gradle sync finished: 1 project, 40 tasks (Gradle 8.14.3)"; tree shows `java-gradle → Tasks → application, build, build setup, distribution, documentation, help, other, verification` |
| `gradle/21-gradle-tool-window.png` | 21 + 17 | Reopened (model restored from `.archipel/gradle/model.json` without running Gradle), expanded `application` and `verification`, double-clicked `run` → Run window "java-gradle [Gradle :run]: successful" with the task tree and "Hello, Gradle!" |

Compared with the reference scene 21: same tree shape and lower-case group names. Differences:
the project node icon is an original cube, not the Gradle elephant. There is no Dependencies
node and no Run Configurations node, the toolbar lacks Link/Unlink, Offline mode, Dependency
Analyzer and the settings menu, and a filter field replaces type-to-search.

# Build, Gradle, Maven and external-system specification (clean-room)

Requirements for the Archipel IDE build area, written from observed behaviour, resource bundles and plugin descriptors of IntelliJ IDEA Community (cited as `ic: ...`). No code, comments or structure are copied. Each requirement has a stable ID (`BLD-<AREA>-<n>`), a priority and a one-line acceptance. Executable cases (`cases/*.cases.md`) give concrete state, keys and expected result with exact texts.

## Priority legend

- MUST: needed for the first usable Java/Kotlin release with Gradle and Maven projects.
- SHOULD: expected for parity, scheduled after the MUSTs.
- LATER: specified for completeness or deliberately deferred (the reason is stated).

## Files

| File | Topic | ID prefix | Cases |
|---|---|---|---|
| 01-build-and-make.md | Build/Rebuild/Recompile actions, task routing, status texts, Build tool window, auto-build, delegation overview | BLD-MAKE | cases/01-build-and-make.cases.md |
| 02-external-system-sync.md | Auto-sync, Load Changes button, sync execution and tab, failures, linking, trust, task activation, tool window framework, offline, dependency analyzer | BLD-SYNC | cases/02-external-system-sync.cases.md |
| 03-gradle.md | Gradle settings, tool window, run configuration, Run Anything, test runner, composite builds, source sets, issues and quick fixes | BLD-GRADLE | cases/03-gradle.cases.md |
| 04-maven.md | Maven settings, importing, tool window, delegation, run configuration, sync issues | BLD-MAVEN | cases/04-maven.cases.md |
| 05-wrapper-jdk-and-toolchains.md | JDK references, Gradle JVM resolution, compatibility matrix, JDK messages, wrapper, toolchains | BLD-JDK | cases/05-wrapper-jdk-and-toolchains.cases.md |

Project model terms (modules, SDKs, source roots) are defined in `docs/specs/platform/07-project-structure-and-scopes.md`; this area references, not duplicates, them.

## Inventory coverage

Every item of `docs/specs/inventory/build.md`, `gradle.md`, `maven.md` is covered or marked LATER with a reason: see section "Inventory coverage" in spec 01 (build, Ant, compilation charts), section 12 of spec 02 (external-system actions), section 12 of spec 03 (Gradle) and section 8 of spec 04 (Maven).

## Counts

See the tables below (refresh with a grep for `BLD-... MUST|SHOULD|LATER`).

| Priority | Count |
|---|---|
| MUST | 123 |
| SHOULD | 62 |
| LATER | 6 |

Per file (MUST/SHOULD/LATER): 01 36/9/0; 02 30/13/1; 03 22/17/3; 04 23/16/1; 05 12/7/1. Cases: 295 (01: 64, 02: 65, 03: 58, 04: 65, 05: 44).

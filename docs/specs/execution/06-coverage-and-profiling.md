# 06. Coverage and profiling

Status: specification (mostly `SHOULD` and `LATER`). Requirement prefix `EXE-COV`.
The code coverage plugin (`plugins/coverage-common`, `plugins/coverage`) ships with the Community
edition. The CPU and allocation profilers, the `Profiler` tool window and the `Run with Profiler`
executors are Ultimate-only; they are `LATER` here with that note. Cases: `cases/06-coverage.cases.md`.

ic: `plugins/coverage-common/src/com/intellij/coverage/{CoverageOptions,CoverageOptionsProvider}.java`,
`plugins/coverage-common/resources/intellij.platform.coverage.xml`,
`plugins/coverage-common/resources/messages/CoverageBundle.properties` (copied to `plugins/coverage-common`),
`plugins/coverage/resources/messages/JavaCoverageBundle.properties`,
`platform/platform-resources/src/keymaps/$default.xml` (`SwitchCoverage`).

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-COV-001 | SHOULD | A third executor `Coverage` adds `Run '<name>' with Co_verage` (mnemonic `v`; without a name `Run with Co_verage`; description `Run selected configuration with coverage enabled`) to the Run menu and context menus after `Debug`; no default shortcut. The configuration editor of a coverage-capable type shows a `Code Coverage` tab with the runner choice (`{0} Coverage`, for Java `IntelliJ IDEA`, `JaCoCo`) and include/exclude patterns. | Run menu lists the item for an Application. |
| EXE-COV-002 | SHOULD | When the run ends with coverage data, the *new data* option decides (settings page `Coverage`, section `When New Coverage Is Gathered`): `Show options before applying coverage to the editor` (default; dialog `Do you want to display coverage data for '{0}'?` with buttons `Replace Active Suites`, `Add to Active Suites`, `Do Not Apply Collected Coverage`), `Replace active suites with the new one`, `Add to the active suites`, `Do not apply collected coverage`. | Default prompts with the dialog. |
| EXE-COV-003 | SHOULD | Settings `Activate Coverage View` (default on) shows the `Coverage` tool window (right side, secondary) after a run; `Show coverage in the project view` (default on) adds a statistics column to the project tree (`% lines covered`, files `{0}% files`). | After a run the Coverage window opens. |
| EXE-COV-004 | SHOULD | Editor gutter: covered lines green, partially covered (branches) yellow, uncovered red; the gutter popup shows `Hits: {0}` and for branches `Covered {0}/{1} branches`; next/previous coverage marks (`Next Coverage Mark`, `Previous Coverage Mark`). | A line executed three times shows `Hits: 3`. |
| EXE-COV-005 | SHOULD | Coverage tool window: tree of packages (root `all`) with columns `Element` and `Statistics, %` (classes, methods, lines; `% lines covered`); toolbar filters (`Flatten packages`, `Hide Fully Covered {0}`, `Show Only {0} with Uncommitted Changes`), `Generate Coverage Report`, `Hide Coverage`; empty text `No coverage results.` | Tree shows per-package percentages. |
| EXE-COV-006 | SHOULD | Actions: `Show Code Coverage Data` (`Ctrl+Alt+F6`; chooser titled `Choose Coverage Suite to Display`, empty `No coverage suites configured.`), `Hide Coverage`, `Generate Coverage Report` (`Generate Coverage Report for: '{0}'`), `Import External Coverage Report...`. | `Ctrl+Alt+F6` opens the chooser. |
| EXE-COV-007 | LATER | `Show Tests Covering Line` (`Show tests covering line`, `Tests Covering Line {0}:{1}:`), per-test coverage, `Run Coverage` via context (`RunCoverage` has no default shortcut), Python/other-language runners. | Not applicable. |
| EXE-COV-008 | LATER | Profilers: CPU/allocation profiling, `Run '<name>' with Profiler`, `Profiler` tool window, flame graphs. Ultimate-only; not in Community. | Not applicable. |

Open questions: the exact default colors of the coverage gutter belong to the color-scheme spec; the
JaCoCo runner availability depends on the plugin set; the `Coverage` executor is registered by the
plugin, not the platform.

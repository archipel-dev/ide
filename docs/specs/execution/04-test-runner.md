# 04. Test runner (SM runner tree, results, rerun, export, history)

Status: specification. Requirement prefix `EXE-TEST`. Cases: `cases/04-test-runner.cases.md`.
Test configurations and their creation from context are in `01` (`EXE-CONF-052/053`) and `02`
(`EXE-CTX-035..037`, gutter markers `EXE-CTX-050..055`). Archipel runs tests through its own
runner that must speak the event protocol of section 2 so the same UI serves every framework.

ic: `platform/smRunner/src/com/intellij/execution/testframework/sm/runner/{SMTestProxy,OutputToGeneralTestEventsConverter,GeneralToSMTRunnerEventsConvertor,GeneralIdBasedToSMTRunnerEventsConvertor}.java`,
`.../sm/runner/events/*`, `.../sm/runner/states/*`, `.../sm/runner/ui/{TestsPresentationUtil,SMTestRunnerResultsForm,SMTRunnerConsoleView,TestTreeRenderer}.java`,
`.../sm/runner/history/actions/*`, `.../sm/TestHistoryConfiguration.java`,
`platform/testRunner/src/com/intellij/execution/testframework/{TestConsoleProperties,ToolbarPanel,TestsUIUtil,TestFrameworkRunningModel}.java`,
`.../testframework/actions/{AbstractRerunFailedTestsAction,ViewAssertEqualsDiffAction,ScrollToTestSourceAction}.java`,
`.../testframework/autotest/*`, `.../testframework/export/*`,
`platform/execution-impl/src/com/intellij/execution/{TestStateStorage.java,runners/RerunTestsAction.java}`,
`platform/smRunner/resources/messages/SmRunnerBundle.properties`,
`platform/testRunner/resources/messages/TestRunnerBundle.properties`,
`platform/execution/resources/messages/ExecutionBundle.properties`,
`plugins/junit/src/com/intellij/execution/junit2/ui/actions/RerunFailedTestsAction.java`.

## 1. Model: test nodes, statuses, counters

A **test node** is a *suite* (has children) or a *test* (leaf). Fields: name, location hint (url
that navigates to the source), duration (milliseconds, may be unknown), state, output (ordered
text pieces with type stdout/stderr/system), and failure data (message, stack trace, expected and
actual text, error vs failure, optional file paths for expected/actual).

States (**magnitudes**, with default tree icon meaning):

| State | Title (bundle) | Leaf/suite | Defect? |
|---|---|---|---|
| Not run | `Not run` | both | no |
| Running | `Running...` | both | no |
| Passed | `Passed` | both | no |
| Completed (suite finished, children not all passed) | `Completed` | suite | no |
| Skipped | `Skipped` | leaf | no |
| Ignored | `Ignored` | leaf | no |
| Failed (assertion) | `Assertion failed` | leaf, suite | yes |
| Error | `Error` | leaf, suite | yes |
| Terminated | `Terminated` | both | yes |

A suite's state aggregates its children: any defect child makes it failed/error; otherwise ignored
only when all finished children passed or ignored and at least one ignored; otherwise passed;
a suite in progress is running. The root node aggregates the whole run.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-TEST-001 | MUST | Node model and statuses above, with aggregation. Accessible status strings: `Error`, `Failed`, `Failed with errors`, `Ignored`, `Ignored with errors`, `Passed`, `Passed with ignored`, `Passed with errors`, `Not ran`, `Running`, `Running with errors`, `Paused`, `Paused with errors`, `Skipped`, `Skipped with errors`, `Terminated`, `Terminated with errors`. | Suite with one failed and two passed children is failed. Case EXE-TEST-001 case 1. |
| EXE-TEST-002 | MUST | Durations: a leaf's duration is reported by the framework (finish event). A suite's duration is the sum of its children's known durations (unknown if none known) unless the framework declares explicit suite durations. Rendered with the approximate narrow duration format (for example `12 ms`, `1 s 234 ms`); a running node shows a padded live timer. Passed leaves show the duration only when not hidden. | Leaf with 12 ms shows `12 ms`. |
| EXE-TEST-003 | MUST | Counters: *total* (announced count, may be unknown while running), *finished*, *failed* (defects), *ignored*, *passed*, *not started* (announced but not finished at the end). | Case EXE-TEST-003 case 1. |
| EXE-TEST-004 | MUST | The progress line while the run is in progress: `Running: <done> of <total>` where `<total>` is the announced count or the placeholder `<...>` while unknown and not finished; with failures `  Failed: N`; categories (non-default test categories, joined by `, `, first letter of the first kept uppercase, others lowercase) follow `Running:`/`Done:`; after the end `Done: <n> of <total>  (<duration>)` (two spaces before the parenthesis); the line ends with two spaces. | `Running: 3 of 10  Failed: 1  `; `Done: 10 of 10  Failed: 1  (2 s 345 ms)  `. |

## 2. Event protocol (framework to IDE)

Output of the test process is scanned for service messages `##teamcity[name 'key'='value' ...]`
(one per line). Values escape `|` as `||`, `'` as `|'`, newline as `|n`, carriage return as `|r`,
`[` as `|[`, `]` as `|]`, and non-ASCII as `|0xXXXX`. Everything else is regular console output
attributed to the running test.

| Message | Attributes | Effect |
|---|---|---|
| `enteredTheMatrix` | | handshake: tells the IDE the test reporter is attached. |
| `testingStarted` / `testingFinished` | | start/end of the run (root running/finished). |
| `rootName` | `name`, optional `location` | label of the root node (shown instead of `Test Results`). |
| `testCount` | `count` | the announced total. |
| `testSuiteStarted` / `testSuiteFinished` | `name`, `locationHint`, `nodeId`, `parentNodeId`, `metainfo` | suite begin/end. |
| `testStarted` / `testFinished` | `name`, `locationHint`, `nodeId`, `parentNodeId`, `duration` (ms) | test begin/end. |
| `testFailed` | `name`, `message`, `details` (stack trace), `error='true'` for errors, `type='comparisonFailure'` with `expected`, `actual`, optional `expectedFile`, `actualFile`, `duration` | marks failure or error. |
| `testIgnored` | `name`, `message` | marks ignored with the reason. |
| `testStdOut` / `testStdErr` | `name`, `out` | attaches output to a test. |
| `suiteTreeStarted`/`suiteTreeNode`/`suiteTreeEnded`/`treeEnded` | `name`, `nodeId`, `parentNodeId`, `locationHint` | pre-declare the tree (so not-started tests are known). |
| `customProgressStatus` | `type`, `count` | custom counters (`testStarted`/`testFinished`, ...). |
| `setNodeProperty` | `nodeId`, `propertyName`, `propertyValue` | per-node properties (for example duration strategy). |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-TEST-010 | MUST | The converter implements the table. Messages are matched per line; a service message in the middle of a line is recognized only when the line starts with it (text before it is output). Unknown messages are ignored. Invalid attribute combinations (finish of an unknown test) do not throw; they are logged and skipped. | Case EXE-TEST-010 case 1. |
| EXE-TEST-011 | MUST | Name-based correlation: without `nodeId` a test is identified by (enclosing open suites, name); a `testFinished` closes the matching started test; a `testFailed` for the currently open test marks it, and a `testFailed` after `testFinished` marks the already finished test failed (late failure). Duplicate names inside one suite create distinct nodes. | Two tests named `t` in a suite are two nodes. |
| EXE-TEST-012 | MUST | When the process ends while a test or suite is running, running nodes become *Terminated* and the root shows `Terminated`; when the process produced no handshake the root says `Test framework quit unexpectedly`; when it started but announced no tests: `No tests were found`. | Case EXE-TEST-012 case 1..2. |
| EXE-TEST-013 | MUST | The number of tests not started (announced via the pre-declared tree but never finished) are counted *not started* and shown with the not-run icon. | Tree declared 5 tests, 3 ran: not started = 2. |
| EXE-TEST-014 | MUST | Output attribution: text received between `testStarted` and `testFinished` belongs to that test; text before the first test belongs to the root. Selecting the root shows all output; selecting a node shows its output. Failure text (message, then stack trace) is appended as error output of the failed test. | Select a test: only its output. |
| EXE-TEST-015 | SHOULD | Location hints are URLs `java:suite://<qualified class>` and `java:test://<qualified class>/<method>` (also `file://<path>:<line>` and `<protocol>://...` per language); resolution happens on demand (double-click/F4). Unresolved: navigation does nothing. | Double-click on a leaf with `java:test://p.FooTest/bar` opens `bar`. |

## 3. Result view (layout and nodes)

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-TEST-020 | MUST | The test run tab of the Run (Debug) window has: toolbar; status line (`Running: ...` text and a progress bar); the **test tree** (left); the **output pane** (right, a console, same behaviors as `03`, including hyperlinks to stack frames; stack trace folding is off there). Splitter proportion is remembered. | A failed run shows the tree and the console of the selected node. |
| EXE-TEST-021 | MUST | Root label: `Test Results` (or the `rootName`); `Instantiating tests...` before anything is known; `Running tests...` while running with children; `Terminated`; `No Test Results`; `All Tests Passed`; `All Tests Passed (except ignored)`; `Test framework quit unexpectedly`; `No tests were found`. Nodes without a name show `<no name>`. | Case EXE-TEST-021 case 1. |
| EXE-TEST-022 | MUST | Tree node text: name; suites show the name of the class (short) with the package as grey comment when the framework supplies it; leaves show the method name; after a space the grey duration (EXE-TEST-002) unless inline statistics are off (`Show Inline Statistics`, default on). Parameterized tests show the parameter text. | Leaf `bar` with 5 ms shows `bar 5 ms`. |
| EXE-TEST-023 | MUST | Icons: passed green check, failed red cross (error: red cross with exclamation), ignored grey/orange crossed, skipped grey, running spinner (animated), not run grey outline, terminated grey stop. Defects bubble to all ancestors. | A failed leaf and all its ancestors show the failure icon. |
| EXE-TEST-024 | MUST | Sorting options (toolbar, mutually exclusive, defaults all off, so the natural order of arrival): `Sort Alphabetically`, `Sort By Declaration Order`, `Sort By Duration`; and `Suites Always on Top` (default on) as a modifier. | Sort alphabetically orders leaves by name; suites stay first. |
| EXE-TEST-025 | MUST | Filters: `Show Passed` (toggle, default shown for Java test runners) and `Show Ignored` (default shown). When passed tests are hidden, passed leaves vanish except those that are defects or that lie below a node still needing display; the root is always visible. Filtering does not change counters. | Hide passed: only failed/ignored/running remain. Case EXE-TEST-025 case 1. |
| EXE-TEST-026 | MUST | `Track Running Test` (default on): while the run is in progress the tree selects the currently running test (and scrolls to it). Manually selecting another node turns tracking off until `Scroll to Running Test` (`Select the currently running test and resume tracking it`) is used. | Selecting a node during the run stops following. |
| EXE-TEST-027 | MUST | `Select First Failed Test When Finished` (default off): at the end the first defect is selected and revealed. When off, the previous selection is kept. Failed suites expand automatically as results arrive; passed suites stay as they are. | Option on: selection ends on the first failed leaf. |
| EXE-TEST-028 | SHOULD | Expand all / collapse all (`Expand all test suites`, `Collapse all test suites`); `Previous Failed Test` / `Next Failed Test` (navigate defects, wrap at ends, select and reveal). | Next at the last failure wraps to the first. |
| EXE-TEST-029 | MUST | Tree and output selection stay in sync: selecting a node shows its output in the right pane; `Open selected test in editor` and double-click/`F4`/`Enter` navigate to the source (EXE-TEST-015). `Autoscroll to Source` (default off) navigates on selection. `Open Source at Exception` (default on) navigates to the first source frame of the stack trace when opening a failed test. | Double-click a failed test with `Open Source at Exception` on lands on the failing line. |
| EXE-TEST-030 | SHOULD | `Scroll To` options for the output pane: `Stack Trace` (scroll to the beginning of the failure's stack trace, default off), `Top` (default) or `Bottom` after the test ends. | With `Stack Trace` on the output scrolls to the first exception line. |

## 4. Failures and diffs

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-TEST-040 | MUST | A failure with `type='comparisonFailure'` adds to the output pane, after the failure message and before the stack trace, the lines `Expected :<expected>` and `Actual   :<actual>` and a link `<Click to see difference>`. Activating the link, or the tree action `View Assert Equals Diff` (shown for such nodes), opens the diff viewer with titles `Expected` (or `Expected (<file url>)` when it comes from a file) on the left and `Actual` (or `Actual (<file url>)`) on the right. | Case EXE-TEST-040 case 1. |
| EXE-TEST-041 | MUST | When no comparison data exists, `View Assert Equals Diff` shows the dialog titled `No Comparison Data Found` with the text `Comparison error was not found`. | Select a passed test, run the action: that dialog. |
| EXE-TEST-042 | MUST | Multiple comparison failures of one test (grouped assertions) are listed so the diff window can step through them in order; the diff window of a string comparison is titled `Comparison Failure` (or `Comparison Failure (<test name>)`). | Two failures: next/previous change in the diff window. |
| EXE-TEST-043 | SHOULD | Failure details parsing: the first stack frame in the project (resolvable class) is the *failure line*; the editor marks it (failed-line marker with the run/debug preview texts `Runs the enclosing test.` and `Debugs the enclosing test and adds a breakpoint on the line of this line.`). | After a failure the source line is marked. |
| EXE-TEST-044 | MUST | Errors (an exception not an assertion) are shown with the error state; the output shows the exception stack trace as error text with hyperlinks per `03`. | Exception test: error icon and linked trace. |

## 5. Rerun, auto-test, history, export

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-TEST-050 | MUST | `Rerun Failed Tests` (toolbar button of the test tab, Run window): enabled when the run finished with at least one failed test; starts a copy of the configuration restricted to the failed tests (leaf nodes of defects; for suites without leaf information the suite). The executor is the one of the tab; with `Shift` held a popup titled `Restart Failed Tests` offers Run/Debug. Not started tests are included when `Include Non-Started Tests in Rerun Failed` (default on) holds. If the failed test set is empty the action is disabled. | Case EXE-TEST-050 case 1. |
| EXE-TEST-051 | MUST | `Rerun Tests` (`Alt+Shift+R`) and the tab's `Rerun` (`Ctrl+F5`) restart the same configuration; the tab is reused and the tree cleared. | After rerun the counters restart from 0. |
| EXE-TEST-052 | SHOULD | Auto-test (`Toggle auto-test`, the clock toggle in the tab toolbar): reruns the configuration after code changes once the project is quiet for the **auto-test delay**, default 3 seconds, selectable 1..10 s (`Set Auto-Test Delay`, items `1s`..`10s`). `Show Auto-Test Status in the Editor` (default on) shows a floating indicator (`Tests Passed`, `Tests Failed`, with `{0}` test name variants). Autotest never starts while the previous run still runs. | Edit a file, wait 3 s: rerun starts. |
| EXE-TEST-053 | MUST | Notifications at the end of a run when the tool window is not visible: a balloon (group `Test Runner`/`Test Results`): all passed `Tests passed: N` (title `Tests Passed`); failures `Tests failed: F, passed: P` with `, ignored: I` and `, not started: S` parts only when non-zero (title `Tests Failed`); only ignored `Tests ignored: I, passed: P` (title `Tests Ignored`); did not start `Failed to start: N, passed: P`; interrupted `Tests were interrupted`; no start `Tests were not started`. A comment `{0}` may follow. The same summary appears in the status bar of the tab as the inline form `F failed, P passed, I ignored, S not started`. | Case EXE-TEST-053 case 1..3. |
| EXE-TEST-054 | SHOULD | Persistent last results: for each test url (`java:suite://...`, `java:test://...`) the IDE stores the last magnitude, date, configuration name, failure info and diff data; the gutter icons (EXE-CTX-051) read it; stored across sessions per project. | Close/open project: markers keep their colors. |
| EXE-TEST-055 | SHOULD | Test history: each finished test run is saved as an XML file in the project's test-results history folder, keeping the last 10 runs (setting default 10; oldest deleted first). `Test History` group (in the tab toolbar import button and in `Run | Test History`) lists the recent sessions as `Open recent test session`; `Import Tests from File...` (`Import the results of a test execution from an XML file`) shows the chooser titled `Choose a File with Tests Result`; import failures show `Import Failed` / `Failed to Parse {0}` / `Unable to parse test results in ''{0}''. Test results are expected to be stored in xml.` | Run tests twice; the history lists 2 sessions. |
| EXE-TEST-056 | SHOULD | `Export Test Results...` (tab toolbar, `More`): dialog titled `Export Test Results` with `Export format` choices `Xml`, a bundled HTML template, `Custom, apply XSL template:`; `Folder:` (default last used), `File name:` (default `Test Results - <configuration name>` with date suffix per the platform), `Open exported file in editor` / `Open exported file in browser`; `Output` group; errors `Output path is empty`, `Output file name is empty`, `User-defined template path is empty`, `User-defined template file ''{0}'' is not found`; overwrite prompt `File ''{0}'' already exists.` + `Do you want to overwrite it?` with button `Overwrite`; progress `Exporting test results`; success balloon `Test results exported successfully to <link>`; failure `Test results export failed: {0}`; footer `Generated by {0} on {1}`. XML root `testrun` with `name`, `duration`, `footerText`, children per test (`test` with `name`, `status`, `duration`, `output`). | Export HTML opens in the browser. |
| EXE-TEST-057 | SHOULD | Test run with coverage/debug uses the same tab; with Debug, the `Console` shows the debugger console and the tests tab is the first content tab; breakpoints suspend the run and mark the root `Paused`. | Debug a failing test: tree shows `Paused` state while suspended. |

## 6. Java frameworks

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-TEST-060 | MUST | JUnit 3/4/5 are supported by the JUnit configuration; the runner process prints the protocol of section 2 (the IDE ships a small runtime for this). Java runs default: `Show Passed` on; `Hide Successful setUp/tearDown` default on; stack trace frames from the testing framework internals are filtered from failure details. | Passing run shows all passed tests. |
| EXE-TEST-061 | SHOULD | TestNG: same protocol with `testng` nodes (suites, groups, tests, methods) and the extra `Hide Successful setUp/tearDown` option. | A failing TestNG method shows in the tree. |
| EXE-TEST-062 | SHOULD | Gradle/Maven delegated test runs feed the same tree through their build tool output (a Gradle test configuration uses the Gradle test events). The specifics belong to the build specs. | A Gradle test task shows the tree. |
| EXE-TEST-063 | MUST | While the project is indexing the run of tests by pattern/package waits or prompts (EXE-CONF-076); in dumb mode the class/method kinds start directly. | Package run in dumb mode shows the partial-index prompt. |
| EXE-TEST-064 | LATER | Test discovery (`Run tests affected by changes`, `Show tests covering line`), `View Report as Test Results` for JUnit XML in the project view, `Add to JUnit Pattern Suite`/`Exclude from Suite`, `Recent Tests` (`Ctrl+Shift+;`), `Find Unused Test Data`. | Not applicable. |

## 7. Open questions

- Exact output of the narrow duration formatter for hours and fractional seconds (observed
  `1 s 234 ms` style; confirm against the running IDE).
- The default export file name pattern (dates) is built by the platform; confirm with a run.
- Test tree icon art (shape and colors) belongs to the icon spec.

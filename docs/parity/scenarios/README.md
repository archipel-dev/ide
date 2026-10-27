# Behavior scenarios

Behavior is specified by recorded scenarios, not by memory. Each scenario types real
keystrokes into a freshly started IDE and compares what the editor shows with what IntelliJ
IDEA shows for the same input. The result is a table of pass, partial, fail and skipped, and
one number: how close the IDE is to IntelliJ.

```bash
cargo run -p xtask -- scenario                         # all scenarios; starts Xvfb when DISPLAY is unset
cargo run -p xtask -- scenario --only java             # an area
cargo run -p xtask -- scenario --only rename-method,find-usages-method
cargo run -p xtask -- scenario --list
cargo run -p xtask -- scenario --install-toolchains    # install missing jdk, jdtls, kotlin-lsp, ... first
cargo run -p xtask -- scenario --report docs/parity/scenarios/REPORT.md   # the committed baseline
```

`--strict` exits non-zero when a scenario fails or only partially matches (the CI job runs it
with `continue-on-error`). The report is printed and written to `target/scenarios/report.md`
(and `report.json`); each scenario's app log, `observed.json` (what the steps saw) and fixture
copy stay in `target/scenarios/work/<area>-<name>/`.

The runner reuses the scenes infrastructure (`xtask/src/scenes.rs`): Xvfb, the glow renderer,
`ARCHIPEL_NO_UPDATE_CHECK=1`, `ARCHIPEL_NO_TOOLCHAIN_SETUP=1`, one IDE process per scenario and
the control socket (`docs/parity/evidence/ui-control-socket.md`). The input is real keyboard
input; every read is a query. Each IDE gets its own `XDG_CACHE_HOME` and `XDG_CONFIG_HOME`, so
language-server workspaces and settings of a scenario never leak into the next one or into the
user's profile. Toolchains stay shared, so they are installed once.

Only text is committed here: markup, popup items and resulting files. No screenshots or artwork
of JetBrains products.

## Layout

```
docs/parity/scenarios/<area>/<name>/
  scenario.json
  project/          a small, real project (build files included); copied and committed to a
                    fresh Git repository for every run
```

## scenario.json

```json
{
  "title": "Static import of assertThat offered for an unresolved call",
  "requires": ["jdk", "jdtls", "gradle", "network"],
  "open": "src/test/java/demo/FooTest.java",
  "steps": [
    {"op": "waitServer", "timeoutMs": 300000},
    {"op": "waitAnalysis", "name": "initial"},
    {"op": "typeAt", "line": 8, "column": 19, "text": "\nassertThat(x).isEqualTo(1);"},
    {"op": "waitAnalysis", "name": "typed"},
    {"op": "caret", "line": 9, "column": 12},
    {"op": "altEnter"},
    {"op": "applyIntention", "title": "re:^(Static import method|Import static method)"}
  ],
  "expect": {
    "markup": [{"when": "typed", "range": "9:9-9:19", "severity": "error",
                "message": "Cannot resolve method 'assertThat' in 'FooTest'"}],
    "intentions": [{"mode": "contains", "items": ["re:^(Static import method|Import static method)"]}],
    "file": [{"contains": ["import static org.assertj.core.api.Assertions.assertThat;"]}]
  },
  "intellij": {"version": "IntelliJ IDEA 2025.2", "action": "Alt+Enter on the call", "notes": "..."},
  "knownGap": "why it differs today (shown in the report)"
}
```

Positions are 1-based like the status bar (`line:column`); a range is `line:column-line:column`
with an exclusive end, or `line:column-column` on one line.

### requires

Toolchain component ids (`jdk`, `jdtls`, `gradle`, `maven`, `kotlin-lsp`, `rust-analyzer`,
`bazelisk`) located through the IDE's toolchain store, or `network` (Maven Central reachable).
A scenario whose requirements are missing is `skipped`, not failed. `--install-toolchains`
installs what is missing through the same store the IDE uses (`toolchain.install`).

When `jdtls` is required the runner starts the IDEs with `JAVA_HOME` set to the newest JDK of
the toolchain store (or `$ARCHIPEL_SCENARIO_JAVA_HOME`): the IDE prefers `JAVA_HOME` over its
managed JDK and a current jdtls needs a newer JDK than most machines have.

### steps

Real input:

| op | does |
|---|---|
| `typeAt {line, column, text}` | moves the caret, then types `text`; `\n` is the Enter key (auto-indent applies) |
| `text {text}` | types at the caret |
| `key {chord}` | presses a chord (`Ctrl+Alt+O`) |
| `caret {line, column}` | moves the caret (what a click does) |
| `altEnter {name?}` | presses Alt+Enter and records the popup rows (ordered as shown) |
| `applyIntention {title \| index}` | chooses a row with Down and Enter; `title` may be `re:<regex>` |
| `complete {name?}` | Ctrl+Space; records the completion rows |
| `declaration {name?}` | Ctrl+B; records the file and caret where it lands |
| `findUsages {name?}` | Alt+F7; records the usages |
| `rename {to}` | Shift+F6, types the new name, Enter |
| `save` | Ctrl+S |

Waiting and setup:

| op | does |
|---|---|
| `waitServer {timeoutMs}` | until the language servers are ready and report no progress |
| `waitAnalysis {name?, timeoutMs}` | until the analysis of the active file settled (`wait analysis`), then records its markup under `name` |
| `sync {command, timeoutMs}` | runs a build-system sync (`gradle.sync`, `bazel.sync`) and waits for it |
| `openFile {path}` | opens another project file |

Every scene step of `docs/parity/scenes/README.md` works too (`key`, `type`, `wait`, `sleep`,
`execute`, `writeFile`, `screenshot`). A step that fails stops the scenario; what was observed
before still counts and the failed step is one more failing check.

### expect

All keys are optional lists; each entry may say `"when": "<name>"` to refer to a named
observation (default: the first) and `"confidence": "unverified"` when the expectation is not
certain.

| key | checks | pass / partial / fail |
|---|---|---|
| `markup` | `range`, `severity` (`error`, `warning`, `weak_warning`), optional IntelliJ `message` | exact / right range and severity, other message (or greyed-out hint instead of warning) / otherwise |
| `noMarkup` | no markup of `severity` (default `error`) in the file | none / - / some |
| `intentions` | `items` in the Alt+Enter popup; `mode` `contains`, `ordered`, `exact`; `re:` prefix for a regex | all / some, or all in another order / none |
| `file` | text of a project file at the end: `equals`, `contains`, `notContains`, `regex` | all / - / otherwise |
| `navigation` | file (`path`, `pathContains`), `line`, `column` after go to declaration | all / right file, other position / otherwise |
| `usages` | `count`, `atLeast`, `paths` | all / some usages / none |
| `completion` | `contains` labels, `first` | all / some / none |
| `state` | JSON pointer into the control socket `state` snapshot: `equals`, `contains` | |
| `query` | runs a command, then checks a pointer into its result: `equals`, `contains`, `atLeast` | |

The scenario status is the worst of its checks. The score is `(pass + partial / 2) / ran`.

### intellij

`version`, `action` and `notes` record how the expected result was observed. Expectations
without a `confidence` are taken from IntelliJ's message bundles and documented behavior and
are the ones to trust; `"confidence": "unverified"` marks what is recalled and still to be
confirmed by hand in IntelliJ. The report lists them with `(unverified)`.

## Queries the runner uses

Besides `state` and `wait`, the runner reads the IDE through ordinary commands of
`archipel-ide-command`: `automation.markup` (markup of the active file as the editor shows it,
1-based ranges), `automation.completion` (the popup rows), `automation.documentText` (text of a
project file; the editor's version wins over disk), `code.alerts` (Alt+Enter rows),
`navigate.results`, `language.servers`, `toolchain.locate`, `gradle.refresh`, `bazel.targets`.
`wait analysis` is a control socket wait condition; the state snapshot reports
`popups.intentions` and `popups.rename`.

## Adding a scenario

1. Create `docs/parity/scenarios/<area>/<name>/project/` with the smallest real project that
   shows the behavior (build files included, a `.gitignore` for build output).
2. Record the behavior in IntelliJ: the markup range and message, the Alt+Enter items, the
   resulting file. Write it into `expect`; mark what you are not sure of `unverified`.
3. Run `cargo run -p xtask -- scenario --only <name>` and read `observed.json` next to the
   app log in `target/scenarios/work/`.
4. Expectations that fail today are fine: that is the measurement. Say why in `knownGap`.

## Corpus

See `REPORT.md` for the current result per scenario.

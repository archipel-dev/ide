# Execution specification (clean-room): run, debug and tests

Requirements for running configurations, the console, the test runner, the debugger and coverage in
the Archipel IDE, written from the observable behavior and the public message bundles and keymaps
of IntelliJ IDEA Community (cited as `ic: ...`). No code, comments or structure are copied.
Prefix `EXE-`; each requirement has an ID, a priority and an acceptance criterion; executable
acceptance cases live in `cases/` and use the block format of the specification brief
(`### <ID> case <n>: title`, `settings`, `language`, `before`, `keys`, `after`).

Saving open documents before a launch is specified in `docs/specs/platform/09-saving-and-autosave.md`
(`PLAT-SAVE-030`..`PLAT-SAVE-035`) and is only referenced.

## Files

| File | Topic | ID prefix |
|---|---|---|
| 01-run-configurations.md | Model, names, temporary/permanent, storage, templates, Edit Configurations, types, before-launch, environment | EXE-CONF |
| 02-run-from-context.md | Run menu and shortcuts, launch lifecycle, rerun/stop, producers, gutter markers, selector and chooser | EXE-CTX |
| 03-run-tool-window-and-console.md | Tool window tabs, console output, ANSI, hyperlinks, folding, input, scrolling | EXE-CON |
| 04-test-runner.md | Test tree, statuses, event protocol, filters, diffs, rerun failed, history, export | EXE-TEST |
| 05-debugger-core.md | Sessions, breakpoints, stepping, frames, variables, watches, evaluation, HotSwap, JDWP behavior | EXE-DBG |
| 06-coverage-and-profiling.md | Coverage (Community), profilers (Ultimate, LATER) | EXE-COV |
| 07-inventory-coverage.md | Mapping of every inventoried action/page/type to a requirement or LATER | EXE-INV |
| cases/*.cases.md | Executable acceptance cases | |

## Counts (MUST / SHOULD / LATER)

01: 43/13/2; 02: 34/12/1; 03: 23/13/1; 04: 27/11/1; 05: 56/14/2; 06: 0/6/2; 07: 0/0/8.
Total 183 / 69 / 17. Cases: 111.

## Open questions

- Confirmation text when closing a tab of a running process; exact narrow duration formatter;
  macOS values of the debug shortcuts; global next/previous frame shortcuts; default for `Focus
  application on breakpoint`; whether the Java debugger always listens (`server=n`) for launched VMs.

# Java acceptance cases: format

One file per spec file, named `<NN>-<name>.cases.md`, in the block format of
`docs/specs/editor/cases/README.md` (`### <ID> case <n>: <title>`, `settings:`, `language:`, a `before`
block, `keys:`, an `after` block). `<caret>` and `<selection>...</selection>` markers, key syntax
(`Alt+Enter`, `type "x"`, `action <ActionId>`) and the optional `status: unverified (<reason>)` line are the same as
there. Java-specific conventions:

* `language: java` is the normal case; the `before` text is a complete compilation unit or the part of one
  that is shown. Several files are separated by `--- path/File.java` lines; the caret file is the one that holds
  `<caret>` (the first one when none does).
* **Inspection cases (files `09-inspections-*`)**
  * `keys: Alt+Enter, choose "<fix name>"` presses Alt+Enter with the caret on the highlighted element
    (or inside it) and picks the entry with exactly that text; `after` is the fixed text. Fixes that start a
    refactoring continue with the keys listed after the choice (for example `type "NewName", Enter`).
  * `keys: inspect` runs the inspection over the file as the daemon would. `after` then does not repeat
    the code but lists the expected highlights, one per line:
    `problem: <level> <highlighted text> "<message>"`. Levels are `error`, `warning`, `weak warning`,
    `info` (no marker, intention only) and `text attributes`. The highlighted text is the exact range.
  * `after` = `no problem reported` means the inspection must produce no highlight and no fix on the
    caret element. `no quick fix "<name>" in the popup` means the named fix must not be offered.
  * `keys: inspect batch` runs the global (whole-project) inspection as *Analyze | Inspect Code* does; only the
    listed `problem:` lines appear, there is no editor highlight.
  * In multi-file cases a separator `--- library: <path>` marks a file that belongs to a library (class files
    root) instead of project sources.
  * `settings:` names inspection options by the label shown in *Settings | Editor | Inspections* and the project
    setup: `library=junit3,junit4,junit5,hamcrest,testng,jetbrains-annotations,checker-qual` puts those libraries on the module classpath;
    `language level=N` sets the module language level; `JDK=N` the project JDK; `dependency rule=<text>` a dependency
    rule of the Scopes settings; `<option label>=on|off|<number>` sets an option of the
    inspection under test. Unlisted options keep their defaults.
  * Only the inspection named by the requirement is enabled in the profile unless `settings:` says otherwise;
    inspections that are off by default are switched on for their cases.
* Message texts are the final texts after argument substitution; `#ref` in a bundle template becomes the
  highlighted text.

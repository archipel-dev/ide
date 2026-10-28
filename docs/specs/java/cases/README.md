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
  * `keys: inspect` runs the inspection over the file as the daemon would (highlight only, no fix applied).
    `after` then does not repeat the code but lists the expected highlights, one per line, in exactly this form:
    `problem: <level> `<highlighted text>` "<message>"`. Levels are written in lower case: `error`, `warning`,
    `weak warning`, `info` (no marker, intention only) and `text attributes` (a styling-only highlight such as
    greyed-out code or a reassigned-variable underline; its message is empty: `""`). The text between the
    backticks is the exact highlighted range, copied from the `before` text without the caret marker. When a
    range cannot be written as source text (an element of a module or package, or a library), the level is
    followed by the element kind and its name in backticks: `problem: warning module `app` "..."`,
    `problem: warning package `p` "..."`, `problem: warning library `libfoo` "..."`. A case with several highlights
    lists them in editor order. The message is the final text after argument substitution; when
    its argument text could not be confirmed the case carries a `status: unverified (...)` line.
  * `after` = `no problem reported` means the inspection must produce no highlight and no fix on the
    caret element. `no quick fix "<name>" in the popup` (used with a bare `keys: Alt+Enter`) means the named
    fix must not be offered at the caret.
  * `keys: inspect batch` runs the global (whole-project) inspection as *Analyze | Inspect Code* does over every
    file of the case; only the listed `problem:` lines appear (same line format as above), there is no editor
    highlight, and no quick fix is applied. Inspections that are only available in batch mode (the "global"
    inspections of the catalog) use this form; a `<caret>` in a batch case only marks the element of interest.
    A batch case for an inspection
    that is off by default names it in `settings:` (`enable inspection <ShortName>`).
  * **Multi-file and multi-module cases.** Several files are separated by `--- <path>` lines inside the `before`
    block (the file that holds `<caret>` is the one the keys act on). Cases that need a module structure start
    the `before` block with one `modules:` line and then give the files with the module directory as first
    path segment: `modules: app (depends on lib), lib` followed by `--- app/A.java` and `--- lib/lib/L.java`.
    Inside the `modules:` line, `(depends on <module>, ...)` adds module dependencies, `(language level N)`
    sets the module language level and `(library: <jar>)` puts a library on the module. Module names in the
    list are separated by commas outside the parentheses. A `module-info.java` file is written as
    `--- module-info.java` (single-module cases). The same separators are used in `after` blocks that show
    changed files.
  * `settings:` names inspection options by the label shown in *Settings | Editor | Inspections* and the project
    setup: `library=junit3,junit4,junit5,hamcrest,testng,jetbrains-annotations,checker-qual` puts those libraries on the module classpath;
    `language level=N` sets the module language level; `JDK=N` the project JDK; `dependency rule=<text>` a dependency
    rule of the Scopes settings; `<option label>=on|off|<number>` sets an option of the
    inspection under test. Unlisted options keep their defaults.
  * Only the inspection named by the requirement is enabled in the profile unless `settings:` says otherwise;
    inspections that are off by default are switched on for their cases.
* Message texts are the final texts after argument substitution; `#ref` in a bundle template becomes the
  highlighted text.
* **Provenance of the examples.** All code in the cases is written for this spec. Message texts, option labels and
  fix names are quoted from the IDE's message bundles (short facts); identifiers, domains and code shapes of the
  examples are our own and are not taken from the IDE's inspection descriptions or test data.

# Editor specification (clean-room)

Requirements for the Archipel IDE text editor, written from the observable behavior and the public
API contracts of IntelliJ IDEA Community (source basis: the sparse clone of `intellij-community`;
cited as `ic: <path>`), and from its message bundles, registry, advanced settings and default keymap.
No code, comments or structure are copied. Each requirement has a stable ID, a priority and an
acceptance criterion; the precise ones are executable **cases** (before text, keys, after text).
Product direction: analysis runs in Archipel's own engine; the specs describe IntelliJ's observable
behavior and contracts independent of implementation language.

## Priority legend

- MUST: needed for the first usable editor that feels like IntelliJ in everyday Java and Kotlin editing.
- SHOULD: expected for parity, scheduled after the MUSTs.
- LATER: specified for completeness, not scheduled.

## Files

| File | Topic | ID prefix | MUST | SHOULD | LATER | Cases |
|---|---|---|---|---|---|---|
| [01-caret-selection-and-movement.md](01-caret-selection-and-movement.md) | caret and selection model, word/camel-hump/line/page movement, smart Home and End, Extend and Shrink Selection, column selection, multiple carets, Escape, caret settings | `ED-CARET` | 46 | 11 | 1 | 71 |
| [02-typing-and-typed-handlers.md](02-typing-and-typed-handlers.md) | typing pipeline, bracket and quote pairing, overtype, tab-out, surround on typing, Backspace and Delete, Enter (indent, braces, comments), Tab and Shift+Tab, Complete Current Statement, paste | `ED-TYPE` | 57 | 8 | 0 | 106 |
| [03-line-and-text-operations.md](03-line-and-text-operations.md) | duplicate, delete line, join lines, move line/statement/element, line and block comment, toggle case, transpose, sort/reverse/unique lines, copy and cut | `ED-LINE`, `ED-CLIP` | 20 | 6 | 0 | 66 |
| [04-folding.md](04-folding.md) | fold model, update rules, every folding command and its selection rule, defaults, Java regions and placeholders, custom regions, persistence | `ED-FOLD` | 28 | 9 | 1 | 30 |
| [05-undo-redo.md](05-undo-redo.md) | command grouping and merging, undo and redo with editor-state restore, global undo, limits, dialogs | `ED-UNDO` | 22 | 5 | 0 | 28 |
| [06-live-templates-and-postfix.md](06-live-templates-and-postfix.md) | template syntax and variables, predefined functions, expansion and sessions, Insert Live Template, Surround With, postfix, Java catalog | `ED-TMPL`, `ED-POST` | 31 | 6 | 0 | 30 |
| [07-editor-appearance-and-settings.md](07-editor-appearance-and-settings.md) | defaults of every Editor, General setting, gutter, caret and highlighting, scrolling, soft wraps, whitespace, guides, breadcrumbs, sticky lines, inlay hints, zoom, timings | `ED-APPEAR` | 24 | 12 | 0 | 7 |
| [08-tabs-and-editor-management.md](08-tabs-and-editor-management.md) | opening, tab placement, closing and the active tab, tab limit, preview and pinned tabs, reopen, splits, editor history, Back/Forward, Recent Files and Locations | `ED-TABS`, `ED-NAV` | 22 | 6 | 0 | 28 |
| [09-code-style-interplay.md](09-code-style-interplay.md) | the code style values the editor reads, precedence (EditorConfig, detection, scheme), indent detection | `ED-STYLE` | 8 | 2 | 0 | 6 |
| **Total** | | | **258** | **65** | **2** | **377** |

Counts are of requirement rows with an explicit priority (`grep -E '^\| ED-.* \| (MUST|SHOULD|LATER)'`).
Cases are the `###` blocks of `cases/<NN>-<name>.cases.md`.

## Executable cases

`cases/README.md` defines the block format (`settings`, `language`, before text with `<caret>` and
`<selection>` markers, `keys`, after text). Case files are normative: if a requirement row and a case
disagree, the case wins and the row is a defect to report. A case marked `status: unverified (...)`
was derived from reading the source but not confirmed in a running IDE; implementers still implement
it and reviewers confirm it first. Defaults and keys are those of the default Windows/Linux keymap.

## Reading order for implementers

1. `01` and `02` define the keystroke contract; `03` the commands built on it.
2. `05` explains why undo steps break where they do (this is what makes typing feel right).
3. `04`, `06`, `08` are the larger subsystems; `07` and `09` are the settings and defaults.

## Overlap with other spec sets

- Typed and paste hooks are listed at a high level in `../codeinsight/08-optimize-imports-and-reformat.md`
  (`CI-OPT-060`); file 02 here is the detailed contract. Reformat Code and optimize imports stay there.
- Document, commit model and saving: `../platform/01-vfs-and-documents.md` (and `09-saving-and-autosave.md`
  when present). Completion popups and Alt+Enter: `../codeinsight/06-completion.md`, `04-...`.
- Go to Declaration, Find Usages, Next Error: `../codeinsight/07-navigation.md`.

## Disagreements with existing docs

None found at the time of writing for the editor; the existing `CI-OPT-060` table is consistent with
`ED-TYPE-010` to `ED-TYPE-102` (this set refines, it does not contradict it), except that
`CI-OPT-060` calls "smart backspace" a code insight setting: its UI label is `Unindent on Backspace`
with three values (`ED-TYPE-046`).

## Top open questions

- Caret placement after several commands (Fold Selection, Toggle Case with a caret mid-word, Postfix
  `sout`, Join Lines in Java, Surround with live template) is derived from source but unverified
  (cases marked `status: unverified`).
- Whether the comment toggle defaults of the base code style (first column on, no space) are
  overridden by the shipped Java and Kotlin styles (`ED-LINE-041`).
- Undo: whether Redo restores the editor state before it reapplies a step symmetrically to Undo
  (`ED-UNDO-011`, Redo not read).
- The complete list of commands that mark themselves as navigation for Back/Forward (`ED-NAV-002`).
- The default of the completion auto-popup delay and the distance at which navigation centers the
  target line (`ED-APPEAR-021`).
- Exact Java and Kotlin selection-extender ranges for Extend Selection (`ED-CARET-031`).

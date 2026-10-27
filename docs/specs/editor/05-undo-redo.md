# Editor spec 05: command grouping, undo and redo

Status: specification, language independent. Requirement prefix `ED-UNDO`.
Priorities: `MUST` / `SHOULD` / `LATER`.

Specifies what the editor records as one undoable step, which successive commands merge into one
step, how Undo and Redo restore text **and editor state** (carets, selections, scroll position,
folding), local versus global undo, transparent commands, limits, and the dialogs. The saving of
documents (unsaved state, autosave) is in `platform/09-saving-and-autosave.md` when present; this file
only states that a save does not clear the undo history.

Executable acceptance cases are in `cases/05-undo-redo.cases.md` (format: `cases/README.md`); they
are normative.

ic: `platform/undo/src/com/intellij/openapi/command/impl/{UndoManagerImpl,CommandMerger,UndoableGroup,
UndoRedo,Undo,Redo,UndoDocumentUtil,DocumentUndoProvider}.java`,
`platform/core-api/src/com/intellij/openapi/editor/actionSystem/DocCommandGroupId.java`,
`platform/platform-api/src/com/intellij/openapi/editor/actionSystem/{EditorAction,EditorActionHandler,
EditorWriteActionHandler}.java`, `platform/platform-impl/src/com/intellij/openapi/editor/impl/
DefaultRawTypedHandler.java`, `platform/platform-impl/src/com/intellij/openapi/editor/actions/
{BackspaceAction,TabAction,DeleteToWordBoundaryHandler,EditorActionUtil}.java`,
`platform/lang-impl/src/com/intellij/codeInsight/editorActions/BaseEnterHandler.java`,
`platform/platform-impl/src/com/intellij/openapi/fileEditor/impl/text/TextEditorState.java`,
`platform/util/resources/misc/registry.properties` (`undo.*`),
`platform/platform-api/resources/messages/IdeBundle.properties`,
`platform/platform-resources-en/src/messages/ActionsBundle.properties`,
`platform/platform-impl/resources/intellij.platform.ide.impl.xml` (advanced settings),
`platform/platform-resources/src/keymaps/$default.xml`.

## 1. Model

* A **command** is a named unit of work started by the command service: every editor action
  (typing, Enter, Backspace, Paste, Duplicate, ...) runs as one command, as does a refactoring, a
  quick fix, a mouse caret move (`Move Caret`), a reformat. A command has a **name** (shown in the
  Undo menu text), an optional **group id**, a **confirmation policy**, flags *transparent* and
  *forced global*, the **undoable changes** it made (document edits and special actions), and the
  **editor state before** and **after** it.
* Commands are **merged** into one undoable step (an *undo group*) when they are *compatible*
  (section 3). The undo history of a document is a stack of groups, with a redo stack beside it.
* The **editor state** at the *undo level* consists of: all carets with their selections, the
  distance of the primary caret from the top of the viewport (the scroll position), and the folding
  state. Two states are equal when all three are equal.
* A change is **local** when it affects only the document of one file; it is **global** when the
  group affects several physical files (a rename across files, a refactoring) or was forced global.
  Global groups sit on the stacks of every file they touched.

## 2. Default shortcuts and texts

| Action | Shortcut | Text |
|---|---|---|
| `$Undo` | Ctrl+Z, Alt+Backspace | `Undo {0}`, where `{0}` is the name of the command to undo; `Undo` when it has none (the description reads `last operation`) |
| `$Redo` | Ctrl+Shift+Z, Alt+Shift+Backspace | `Redo {0}` (description `last undone operation`) |

The names of the common editor commands (the text shown after `Undo`): `Typing` (typed characters,
Enter, Tab), `Backspace`, `Paste`, `Cut`, `Delete Line`, `Duplicate Line or Selection`, `Join Lines`,
`Move Line Up`, `Move Line Down`, `Comment with Line Comment`, `Toggle Case`, `Move Caret` (mouse
caret moves). The name is the action's own text from the action bundle.

## 3. Requirements: which commands merge

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-UNDO-001 | MUST | A command merges with the previous recorded step only when both carry the **same non-empty group id**. By default an editor action command gets a fresh *none* group id of its own (it never merges with anything), so each action invocation is its own undo step. | Two consecutive Duplicate commands are two Undo steps. |
| ED-UNDO-002 | MUST | **Typed characters** use the document itself as group id: consecutive typed characters in one document merge into one step regardless of pauses (there is no timer), including the auto-inserted closing bracket or quote, the replaced selection, surround-with-quote on selection and overtype. A typed character that triggers a post-typing reformat (`}` or `)` re-indent, typed `{` at line start) ends the merge: the reformat is its own step. | Type `abc`, Undo: all three letters disappear. |
| ED-UNDO-003 | MUST | Any other command between two typed characters separates them: Enter, Backspace, Delete, Tab, paste, a mouse click (`Move Caret`), **and arrow-key caret moves** (they are commands with a none group id). | Type `ab`, Left, type `x`: Undo removes only `x`. |
| ED-UNDO-004 | MUST | **Enter** has its own group id: consecutive Enter commands merge with each other but not with typing. Newline plus the auto-indent, comment continuation or generated closing brace it produces is one step. | Type `ab`, Enter, Enter, type `c`: three steps (`ab`, two line breaks, `c`). |
| ED-UNDO-005 | MUST | **Backspace** and the word-deletion actions share one *delete group*: consecutive deletions merge into one step; Backspace that joins with the previous line (caret at a line start) resets the group so the join is its own step and the next deletion starts a new step. Tab and the indent actions share an *edit group* (consecutive Tab presses merge). Typing and deleting never merge with each other. | Three Backspaces, Undo: all three characters return. |
| ED-UNDO-006 | MUST | A multi-caret action is one command: one Undo reverts the change at all carets together and restores all carets. | Two carets typing: one Undo reverts both. |
| ED-UNDO-007 | MUST | Merged commands keep the **name of the first command that made a change** and the editor state before of the first and state after of the last. | Type `ab` then Undo shows `Undo Typing`. |
| ED-UNDO-008 | MUST | Each of these is exactly one undoable step: paste together with its reformat and added imports, Enter with smart indent, Reformat Code (including optimize imports when run together), Complete Current Statement, a live template expansion, a refactoring, a quick fix, Duplicate, Delete Line, Join Lines, Move Line/Statement, line comment toggle, Toggle Case, surround. | Paste with `Indent each line`, one Undo restores the text before the paste. |
| ED-UNDO-009 | SHOULD | A command can be **transparent**: its changes merge into the neighbouring step instead of forming their own (used for tooling that should not interrupt typing, for example the on-the-fly optimize imports or automatic indent adjustments). A transparent command touching a different set of documents than its neighbour does not merge. Caret movement is **not** transparent unless the advanced setting `Use undo transparent caret movement` (default **off**) is on. | With the setting on, an arrow key between two typings does not split them. |

## 4. Requirements: undo and redo

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-UNDO-010 | MUST | Undo is available when the focused editor's document (or any global group touching it) has an undoable step on its stack and no step on the way is marked non-undoable; Redo likewise for the redo stack. The menu text and tooltips use the texts of section 2. | Fresh document: Undo disabled. After typing: `Undo Typing`. |
| ED-UNDO-011 | MUST | **Undo first restores the editor state**: if the current editor state (carets, selections, scroll position, folding) differs from the **state after** the step to undo, the first Undo only moves the editor back to that state (caret and selection restored, view scrolled) and does **not** change the text; the recorded state-after is then replaced by the restored one, so the next Undo reverts the text. With `Use undo transparent caret movement` on, both happen in one Undo. | Type `abc`, press Home, Undo: caret returns to the end of `abc`, text unchanged; Undo again removes `abc`. |
| ED-UNDO-012 | MUST | After the text of a step is reverted, the editor state is set to the **state before** the step: all carets and selections as they were when the command began (a replaced selection is selected again), and the viewport scrolls to show the primary caret. Redo restores the state after the step the same way. | Select `bc`, type `x`, Undo: `bc` is selected again. |
| ED-UNDO-013 | MUST | Redo reapplies the most recently undone step. Any new recorded command (a document change in a new command) **clears the redo stack** of that document. Pure caret moves do not clear it. | Type `a`, Undo, type `b`, Redo: nothing happens. |
| ED-UNDO-014 | MUST | Undo and Redo of a step run as one command that is not itself recorded as a new undo step; the redo stack receives the undone step. Text, markers and carets end in exactly the state the matching Redo/Undo expects (undo then redo is the identity on text and state). | Round trip. |
| ED-UNDO-015 | MUST | History limits: per document at most **100** steps (registry `undo.documentUndoLimit`), and at most **10** global steps (`undo.globalUndoLimit`); older steps are dropped from the bottom. | After 150 separate actions only 100 can be undone. |
| ED-UNDO-016 | MUST | Saving a document, reformat-on-save and other save actions do not clear the history; the saved state is only a marker (unsaved flag). Undo after save is allowed and makes the document unsaved again. | Edit, save, Undo: text reverts, document is unsaved. |
| ED-UNDO-017 | MUST | A change to the document that does not come from a command in the IDE (external file change reloaded from disk) invalidates the history for that document: stacks are cleared (or the affected steps marked non-undoable) so that Undo cannot corrupt text. | External change then Undo: Undo is disabled, or reports `Cannot undo`. |
| ED-UNDO-018 | MUST | If the document of a step is read-only, Undo does not change it: a read-only file asks for write access (the version-control prompt); a read-only document fires its modification-attempt notification and the Undo is refused. | Read-only document: Undo leaves text unchanged. |
| ED-UNDO-019 | SHOULD | Undo of a step whose affected range was concurrently changed by a later non-undone step (clash in the same document) is refused with a message instead of guessing: `Cannot Undo` / `Cannot undo`. | Clash case. |

## 5. Local and global undo

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-UNDO-030 | MUST | A step is **global** when it changes more than one physical file or was forced global (file rename and move, multi-file refactoring, project-wide replace). A global step is listed on the stack of every affected file. Undo invoked in an editor takes the last step on **that file's** stack: if it is global, the whole step across files is reverted. | Rename a class (two files change), Undo in either file: both files revert. |
| ED-UNDO-031 | MUST | Before reverting a global step the IDE **asks for confirmation** (default policy for global steps), with dialog title `Undo` (or `Redo`) and message `Undo {command}?`. If other files affected by the step have changed since, Undo refuses and lists them: message `The following files affected by this action have been already changed:`, title `Cannot Undo`; if some affected change cannot be undone: `Cannot perform ''{0}''` followed by `The following files have changes that cannot be undone:`. A conflicting-change confirmation reads `Other files affected by this action have been already changed.` | Undo a rename after editing the other file: refusal listing it. |
| ED-UNDO-032 | SHOULD | When the last step is global but blocked by other changes, Undo may offer to undo only the **local** part of the step: the step is split into a local step named `Local` plus the original name and a remainder; Redo can re-gather them. | Split offered when other files changed. |
| ED-UNDO-033 | SHOULD | Undo in a view with no document (for example a project tree) acts on the global stack (steps without a document affected set). | Undo of a file creation from the tree. |

## 6. Interaction with other editor features

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-UNDO-040 | MUST | Fold regions created by the user (Fold Selection) and the expanded state of regions are part of the editor state; generated regions are re-synced after the text reverts (`ED-FOLD-003`). | Undo after collapse restores the first-undo state restore. |
| ED-UNDO-041 | MUST | Live templates: Undo during an active template exits the template session cleanly; the template text expansion is one step (`ED-TMPL-`). | Expand `sout`, Undo: abbreviation is back. |
| ED-UNDO-042 | MUST | An Undo never leaves auto-inserted pairs half done: typing `(` produced `()` as one step so Undo removes both. | Java: type `(` then Undo: both brackets gone. |
| ED-UNDO-043 | SHOULD | The Undo/Redo actions can be invoked from the Edit menu, the shortcut and the quick-access of the editor; the Edit menu items show the command name. | Menu text `Undo Typing`. |

## 7. Open questions

* Whether a timer or a maximum merged length ever splits a long typing run (none was found in the
  command merger; confirm in a running IDE with a very long run).
* The exact content of the *undo level* of the editor state for folding (all regions or user regions only).
* Behavior of Undo for document changes made by the debugger's hot-swap or VCS operations.

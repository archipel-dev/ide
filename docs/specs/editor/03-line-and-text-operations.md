# Editor spec 03: line and text operations, clipboard

Status: specification, language independent unless a row says Java. Requirement prefixes
`ED-LINE` (line and text commands) and `ED-CLIP` (copy and cut; paste is in file 02 section 8).
Priorities: `MUST` / `SHOULD` / `LATER`.

Commands specified: Duplicate Line or Selection, Delete Line, Join Lines, Move Line Up/Down, Move
Statement Up/Down, Move Element Left/Right, Comment with Line/Block Comment, Toggle Case, Transpose,
Sort/Reverse/Unique Lines, Split Line and Start New Line (specified in file 02), Auto-Indent Lines,
Copy and Cut with and without selection. Typing, Backspace, Enter and Tab are in file 02.

Executable acceptance cases are in `cases/03-line-and-text-operations.cases.md` (format in
`cases/README.md`); they are normative.

ic: `platform/platform-impl/src/com/intellij/openapi/editor/actions/` (`DuplicateAction.java`,
`DuplicateLinesAction.java`, `DeleteLineAction.java`, `ToggleCaseAction.java`, `TransposeAction.kt`,
`AbstractPermuteLinesHandler.java`, `SortLinesAction.java`, `ReverseLinesAction.java`,
`UniqueLinesAction.java`, `CopyAction.java`, `CutAction.java`, `CutLineActionHandler.java`),
`platform/lang-impl/src/com/intellij/codeInsight/editorActions/JoinLinesHandler.java`,
`platform/lang-api/src/com/intellij/codeInsight/editorActions/moveUpDown/{StatementUpDownMover,LineMover}.java`,
`platform/lang-impl/src/com/intellij/codeInsight/editorActions/moveUpDown/{BaseMoveHandler,MoveLineHandler,
MoveStatementHandler,MoverWrapper}.java`, `platform/lang-impl/src/com/intellij/codeInsight/generation/
{CommentByLineCommentHandler,CommentByBlockCommentHandler}.java`,
`platform/code-style-api/src/com/intellij/psi/codeStyle/CommonCodeStyleSettings.java`,
`platform/platform-impl/resources/intellij.platform.ide.impl.xml` (advanced settings),
`platform/platform-resources/src/keymaps/$default.xml`,
`platform/platform-resources-en/src/messages/ActionsBundle.properties`.

## 1. Default shortcuts

| Action (ID) | Shortcut | Text |
|---|---|---|
| EditorDuplicate | Ctrl+D | Duplicate Line or Selection (the text changes to `Duplicate Selection` or `Duplicate Line` by state) |
| EditorDuplicateLines | none | Duplicate Entire Lines |
| EditorDeleteLine | Ctrl+Y | Delete Line |
| EditorJoinLines | Ctrl+Shift+J | Join Lines |
| MoveLineUp / MoveLineDown | Alt+Shift+Up / Alt+Shift+Down | Move Line Up / Down |
| MoveStatementUp / MoveStatementDown | Ctrl+Shift+Up / Ctrl+Shift+Down | Move Statement Up / Down |
| MoveElementLeft / MoveElementRight | Ctrl+Alt+Shift+Left / Right | move the list element at the caret |
| CommentByLineComment | Ctrl+/ (also numpad divide) | Comment with Line Comment |
| CommentByBlockComment | Ctrl+Shift+/ (also numpad divide) | Comment with Block Comment |
| EditorToggleCase | Ctrl+Shift+U | Toggle Case |
| EditorSplitLine | Ctrl+Enter | Split Line |
| EditorStartNewLine / EditorStartNewLineBefore | Shift+Enter / Ctrl+Alt+Enter | Start New Line / Before Current |
| AutoIndentLines | Ctrl+Alt+I | Auto-Indent Lines |
| EditorSortLines, EditorReverseLines, EditorUniqueLines | none | Sort Lines, Reverse Lines, Delete Duplicate Lines |
| EditorTranspose | none | Transpose |
| $Copy / $Cut | Ctrl+C or Ctrl+Insert / Ctrl+X or Shift+Delete | Copy, Cut |
| $Paste | Ctrl+V or Shift+Insert | Paste |
| PasteMultiple | Ctrl+Shift+V or Ctrl+Shift+Insert | Paste from History |
| EditorPasteSimple | Ctrl+Alt+Shift+V | Paste as Plain Text |
| CopyPaths / CopyReference | Ctrl+Shift+C / Ctrl+Alt+Shift+C | not editor commands (project and navigation specs) |

## 2. Duplicate

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-LINE-001 | MUST | **Duplicate Line or Selection**: with no selection, the caret's line (a collapsed fold region on that line counts as part of it, so the whole visual line is duplicated) is copied below itself, and the caret moves into the copy at the same column. If the line has no line break at its end (last line) a break is added before the copy. With a selection (any shape), the selected text is inserted right after the selection end; the **copy** becomes selected and the caret sits at its end. Per caret, processed from the last to the first caret; the action is disabled in one-line editors unless there is a selection. One undo step. | Cases ED-LINE-001. |
| ED-LINE-002 | SHOULD | **Duplicate Entire Lines** (no default key): with a selection duplicates all lines the selection touches (a selection ending exactly at a line start does not include that line) and selects the copy; without a selection behaves like ED-LINE-001's no-selection case. | Case. |

## 3. Delete Line

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-LINE-010 | MUST | **Delete Line** (Ctrl+Y) deletes, for each caret, the logical line(s) it touches: the caret line, or all lines covered by the selection (a selection ending exactly at a line start excludes that last line), including the line break after them. If the last line is deleted (no break after it) the break **before** it is deleted instead. Carets whose ranges touch or overlap are merged into one deletion. Afterward each caret is on the line that took the place of the deleted range (or on the last line), in the **same visual column** it had (clamped to the line length); selections are cleared. Not a kill-ring operation (it resets the clipboard-history grouping). Undo group id is reset so successive Ctrl+Y are separate undo steps. | Cases ED-LINE-010. |
| ED-LINE-011 | MUST | The default shortcut Ctrl+Y is the same key as Redo on some keymaps; the default keymap binds it to Delete Line. A setting-based chooser (an action that asks the user once) is a product detail of the keymap; the kit binds Delete Line. | Ctrl+Y in the default keymap deletes the line. |

## 4. Join Lines

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-LINE-020 | MUST | **Join Lines** (Ctrl+Shift+J) joins the caret line with the next one; with a selection it joins all lines the selection covers (a selection ending at a line start does not include the following line). Without a following line it does nothing. For each break: the line break and the whitespace around it are removed and replaced by **one space**, except when the joined text starts a line end (nothing follows) where no space is used, or where the language's formatter says that a different spacing (including none, for example before `)`) is correct. A whitespace-only first line simply disappears (the next line is re-indented to the code style). | Cases ED-LINE-020. |
| ED-LINE-021 | MUST | Caret after joining without a selection: at the join position (the end of the first line's last non-blank character, before the inserted space). With a selection: at the end of the selection. | Case. |
| ED-LINE-022 | MUST | Language join handlers run before the plain join: (a) raw joiners work on the unmodified text (for example merging two string literals `"a" +` newline `"b"` into one literal, or combining `else` and `if`); (b) non-raw joiners run after the breaks are removed (for example removing the braces of a single-statement block, or merging a declaration with its assignment). A handler returns the caret offset it wants. An **end-of-line comment** on a line followed by code is first converted to a block comment so the code is not commented out. The whole operation runs with a cancellable progress titled `Join Lines` and formatting disabled; it is one undo step. | Java: `if (a)` newline `    b();` joins to `if (a) b();`. |

## 5. Moving lines, statements and elements

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-LINE-030 | MUST | **Move Line Up/Down** (Alt+Shift+Up/Down) swaps the line (or all lines touched by the selection; a selection ending at a line start excludes that line) with the line above or below. Text is moved exactly; **no re-indentation** is applied; the caret and selection move with the text and keep their relative position; collapsed fold regions in the two swapped ranges swap their expansion state. It does nothing at the first line (Up) or at the end of the document (Down). In a multi-caret editor each caret moves its own lines; carets on the same line move it once; if the first caret cannot move, none of the carets moves. Disabled in viewers and one-line editors. | Cases ED-LINE-030. |
| ED-LINE-031 | MUST | **Move Statement Up/Down** (Ctrl+Shift+Up/Down) asks the language's *statement movers* in order; the first that accepts moves a whole syntactic unit (statement, member, XML tag, ...) past the neighbouring unit, may move it **into or out of** an enclosing block, and re-indents the moved range (target, and source when needed) by the code style. If no mover accepts, nothing happens (it does not fall back to a plain line move, except where the language registers a line mover as the last mover). | Java: swaps two statements; moves a statement out of a block over the closing brace. |
| ED-LINE-032 | SHOULD | **Move Element Left/Right** (Ctrl+Alt+Shift+Left/Right) swaps the list element at the caret (argument, parameter, array element, enumerator, ...) with its neighbour, honoring commas; a language handler decides what an element is. | Java: `f(a<caret>, b)` Move Right gives `f(b, a<caret>)`. |

## 6. Comments

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-LINE-040 | MUST | **Comment with Line Comment** (Ctrl+/) toggles line comments on the lines the caret or selection touch (a selection ending at a line start excludes that line; a collapsed fold region extends the range to its whole lines). Decision: if every non-empty line of the range is already commented (and, for a multi-line range, at least one non-empty line is commented) the action **uncomments**; otherwise it **comments**. A single empty line is commented. Language: the commenter of the language at the start of each line decides the prefix; a language with no line comment prefix but with block comment prefix and suffix comments each line as a block comment; if neither exists nothing happens. | Cases ED-LINE-040. |
| ED-LINE-041 | MUST | Placement and spacing come from the code style of the language: `Line comment at first column` (base default **on**) puts the prefix at column 0, off puts it at the indent of the first non-blank line of the range (all lines of the range use that column; blank lines get none); `Add a space at comment start` (base default **off**) controls whether `// ` or `//` is written. When uncommenting, the prefix and a following single space (if spacing is on, or if present) are removed. A language may force indented or first-column style. | Cases show the two defaults. |
| ED-LINE-042 | MUST | **Caret after line comment**: with no selection the caret moves down one visual line after the toggle, keeping its horizontal position (advanced setting `Move caret down after Comment with Line Comment action`, default **on**), except that on an empty line that just received a comment prefix the caret is put right after the prefix. With a selection of whole lines the selection is restored to start at the first line start. When several carets span several lines the carets do not move. | Cases. |
| ED-LINE-043 | MUST | Line comments are never toggled inside an injected fragment that forbids them or whose host text starts with a quote; the host file's commenter is used then. | A fragment in a string uses the host language comment. |
| ED-LINE-044 | MUST | **Comment with Block Comment** (Ctrl+Shift+/): if the caret or selection lies in an existing block comment and the selection does not straddle its edges, the comment is removed (prefix, suffix, and the blanks the action added); if the selection straddles edges or encloses a whole comment, the selection is commented with the comment markers escaped as the language requires (nested block comments are warned: hint text). Without a comment: a selection is wrapped with the prefix and suffix; with no selection the markers are inserted at the caret and the caret is placed between them: `/**/` (no space) or `/*  */` with the caret in the middle when `Add a space ... block comment` is on. Languages whose block comment needs whole lines comment whole lines. | Cases ED-LINE-044. |
| ED-LINE-045 | SHOULD | Defaults for the language comment settings come from `CommonCodeStyleSettings`: line comment at first column on, block comment at first column on, line comment add space off, block comment add space off, add space on reformat off, documentation-line-comment preference off. A language style may override; the product shows them under Code Style, `Code Generation`, `Comment code`. | Settings page shows these values for a language without override. |

## 7. Case, transpose, line permutations

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-LINE-050 | MUST | **Toggle Case** (Ctrl+Shift+U): per caret, a selection or else the word at the caret (camel-hump aware) is converted. The direction is decided once for all carets: if any caret's text contains an upper-case letter it is converted to **lower** case; if all texts are already lower case, to **upper** case (so it is a two-state toggle, not a three-state cycle). Escape sequences in string tokens are not changed. The converted text stays selected; carets keep their positions. | Cases ED-LINE-050. |
| ED-LINE-051 | SHOULD | **Transpose**: with no selections, swaps the two characters around each caret (at a line end: the two before it; at offset 0: nothing) and the caret goes after the pair when it was in the middle of a line; with a selection at every caret and at least two carets, the selected texts are rotated among the carets (each gets the text of the next one, the last gets the first's). | Cases ED-LINE-051. |
| ED-LINE-052 | SHOULD | **Sort Lines**, **Reverse Lines**, **Delete Duplicate Lines**: operate on the selected lines, or on the whole document when there is no selection; need at least two lines; sort is by UTF-16 code unit order of the line text (no locale, case sensitive); reverse reverses the order; unique keeps the first occurrence of equal lines. With a selection the selection becomes the full range of the lines processed ending at the next line start; without one the caret follows its line. | Cases ED-LINE-052. |
| ED-LINE-053 | MUST | **Auto-Indent Lines** (Ctrl+Alt+I) re-indents the caret line or the selected lines to the code style indent (`CI-OPT-043`); the caret stays in its text. | Java: a mis-indented statement gets the block indent. |

## 8. Copy and Cut

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-CLIP-001 | MUST | **Copy** with a selection copies the selected text (all carets: the selected texts joined by `\n`, one per caret, ordered by position; the clipboard also carries a marker of the caret count so that Paste can distribute them). The selection and carets stay as they were. Sticky selection is turned off. | Cases ED-CLIP-001. |
| ED-CLIP-002 | MUST | **Copy with no selection** copies each caret's whole line including its line break (when the line has one) and marks the clipboard "copied from empty selection". The carets and selections are restored afterwards to what they were (the line is selected only for the duration of the copy), except that the copied range is briefly highlighted as a search result (hidden by Escape or any key). Advanced settings: `Don't copy/cut the current line when invoking the Copy or Cut action with no selection` (default **off**), `Don't select the copied line after invoking the Copy action with no selection` (default **off**; when on, the line stays selected after copy). | Cases ED-CLIP-002. |
| ED-CLIP-003 | MUST | **Cut** with a selection copies and deletes the selection at every caret; with no selection cuts the whole line(s) of each caret (including their line break) and marks the clipboard "copied from empty selection". Cut is one undo step. In a read-only editor Cut does not delete (and does not offer the modification dialog beyond the standard one). | Cases ED-CLIP-003. |
| ED-CLIP-004 | MUST | Copy and Cut put plain text on the system clipboard, and rich flavors when available (HTML with syntax colors if the `editor.richcopy` registry (max size 10 MB, strip indents on) allows; the editor-specific flavors: caret count, copied-from-empty-selection, fold regions, indent info, imports). The plain text uses `\n` internally and the platform line separator only for the system flavor. | Round-trip through the clipboard yields the same text. |
| ED-CLIP-005 | SHOULD | Copy and Cut keep a **clipboard history** (see `PasteMultiple`; default limit of the history: 20 entries is an open question). | Ctrl+Shift+V lists recent copies. |
| ED-CLIP-006 | MUST | `Cut up to Line End` and `Cut Line Backward` (no default keys) cut from the caret to the line end / start; a selection is cut as is; at a line end they cut the line break (at the line start backward cuts the break before); the kill-ring chain stops. | Cases in file 02 for the delete variants apply. |

## 9. Open questions

* Whether Move Line on the last line without a trailing line break adds a final line break to the
  document (the implementation inserts a break after a moved range lacking one).
* The clipboard history limit and the structure of the caret-count marker flavor.
* Exact formatter-driven spacing of Join Lines per language (see ED-LINE-020).
* The default values of the comment settings in the shipped Java and Kotlin code styles when a
  newer default overrides the base class (source: base class values).

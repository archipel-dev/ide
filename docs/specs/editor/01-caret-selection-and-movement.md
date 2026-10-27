# Editor spec 01: carets, selection and movement

Status: specification, language independent. Requirement prefix `ED-CARET`.
Priorities: `MUST` / `SHOULD` / `LATER`.

This file specifies where the caret goes and what is selected after every navigation or selection
command of the text editor: character, word and "camel hump" movement, line and text boundaries,
smart Home/End, paging, code block and brace jumps, the selection-extension commands
(Extend/Shrink Selection, Select All, line selection, sticky selection), column (block) selection,
and multiple carets (clone, next occurrence, all occurrences, one caret per selected line,
Escape). Editing commands that merely use a caret (typing, delete, duplicate) are in files 02 and
03. The effect of caret moves on undo is in file 04; folded regions that interact with movement
are in file 03 of the editor set (`03-...` folding) and summarised in section 9 here.

Executable acceptance cases (before text, keys, after text) for every requirement with keys are in
`cases/01-caret-selection-and-movement.cases.md`; the format is in `cases/README.md`. The case files
are normative: where a requirement row and a case disagree, the case wins and the row is a defect.

ic: `platform/platform-impl/src/com/intellij/openapi/editor/actions/` (`EditorActionUtil.java`,
`MoveCaretLeftOrRightHandler.java`, `MoveCaretLeftOrRightWithSelectionHandler.java`,
`MoveCaretUpOrDownHandler.java`, `MoveCaretUpOrDownWithSelectionHandler.java`, `LineStartAction.java`,
`LineEndAction.java`, `Page*Action.java`, `TextStartAction.java`, `TextEndAction.java`,
`SelectLineAction.java`, `ExtendLineSelectionAction.java`, `SwapSelectionBoundariesAction.java`,
`ToggleColumnModeAction.java`, `ToggleStickySelectionModeAction.java`, `ToggleInsertStateAction.java`,
`CloneCaretActionHandler.java`, `AddCaretPerSelectedLineAction.java`, `EscapeAction.java`,
`ScrollToCenterAction.java`), `platform/editor-ui-api/src/com/intellij/openapi/editor/actions/
CaretStopOptions.kt`, `platform/lang-impl/src/com/intellij/openapi/editor/actions/
{SelectOccurrencesActionHandler,SelectNextOccurrenceAction,UnselectPreviousOccurrenceAction,
SelectAllOccurrencesAction}.java`, `platform/lang-impl/src/com/intellij/codeInsight/editorActions/
{SelectWordHandler,UnSelectWordHandler,SelectWordUtil,CodeBlockUtil,MatchBraceAction}.java`,
`platform/lang-impl/src/com/intellij/application/options/editor/EditorCaretStopPolicyItem.kt`,
`platform/ide-core-impl/src/com/intellij/openapi/editor/ex/EditorSettingsExternalizable.java`,
`platform/platform-impl/resources/idea/PlatformActions.xml`,
`platform/platform-resources/src/keymaps/$default.xml`,
`platform/platform-resources-en/src/messages/ActionsBundle.properties`,
`platform/ide-core/resources/messages/ApplicationBundle.properties`,
`platform/util/resources/misc/registry.properties`.

## 1. Vocabulary

| Term | Meaning |
|---|---|
| **Offset** | Index into the document text (`\n` line separators only, see `PLAT-VFS-030`). |
| **Logical position** | (line, column) in the document; a tab counts as the number of columns up to the next tab stop. |
| **Visual position** | (visual line, column) as drawn: soft wraps, folded regions, inlays and virtual lines change the mapping. |
| **Caret** | An insertion point with an optional selection. One caret is the **primary** caret: the one most recently added, which drives scrolling and the caret-related status bar items. |
| **Lead selection offset** | The end of the selection that stays fixed when the selection is extended by moving the caret (the anchor). |
| **Selection** | A start and an end offset (start less than end); the caret sits at one end. A selection may also be a **block selection** (a rectangle of columns over a line range), represented as one selection per line with one caret per line. |
| **Sticky selection** | A mode in which plain movement commands extend the selection as if Shift were held. |
| **Column mode** | A mode in which the caret may be placed after the end of a line and selections are rectangular. |
| **Word boundary** | See section 3. Two flavours exist: plain words, and camel humps. |
| **Smart Home / End** | Home goes to the first non-blank character first; End goes to the end ignoring trailing blanks first (section 4). |

## 2. Default shortcuts of this area (default keymap)

The Windows and Linux column is the `$default` keymap; on macOS the Ctrl modifier is mapped to Cmd
and some moves use Option (the macOS keymaps are out of scope; implementers keep the action IDs and
let the keymap layer decide).

| Action (ID) | Shortcut | Text |
|---|---|---|
| EditorLeft / EditorRight / EditorUp / EditorDown | Left, Right, Up, Down | move one character or line |
| EditorLeftWithSelection etc. | Shift+Left/Right/Up/Down | same, extending the selection |
| EditorPreviousWord / EditorNextWord | Ctrl+Left / Ctrl+Right | Move Caret to Previous/Next Word |
| EditorPreviousWordWithSelection / NextWordWithSelection | Ctrl+Shift+Left / Ctrl+Shift+Right | with selection |
| EditorLineStart / EditorLineEnd | Home / End | Move Caret to Line Start / End |
| EditorLineStartWithSelection / EditorLineEndWithSelection | Shift+Home / Shift+End | with selection |
| EditorTextStart / EditorTextEnd | Ctrl+Home / Ctrl+End | Move Caret to Text Start / End |
| EditorTextStartWithSelection / EditorTextEndWithSelection | Ctrl+Shift+Home / Ctrl+Shift+End | with selection |
| EditorPageUp / EditorPageDown | Page Up / Page Down | scroll one page and move the caret |
| EditorPageUpWithSelection / EditorPageDownWithSelection | Shift+Page Up / Shift+Page Down | with selection |
| EditorMoveToPageTop / EditorMoveToPageBottom | Ctrl+Page Up / Ctrl+Page Down | Move Caret to Page Top / Bottom |
| EditorMoveToPageTopWithSelection / BottomWithSelection | Ctrl+Shift+Page Up / Page Down | with selection |
| EditorCodeBlockStart / EditorCodeBlockEnd | Ctrl+[ / Ctrl+] | Move Caret to Code Block Start / End |
| EditorCodeBlockStartWithSelection / EndWithSelection | Ctrl+Shift+[ / Ctrl+Shift+] | with selection |
| EditorMatchBrace | Ctrl+Shift+M | Move Caret to Matching Brace |
| EditorSelectWord / EditorUnSelectWord | Ctrl+W / Ctrl+Shift+W | Extend Selection / Shrink Selection |
| $SelectAll | Ctrl+A | Select All |
| EditorScrollUp / EditorScrollDown | Ctrl+Up / Ctrl+Down | Scroll Up / Down (caret stays; moves only if it leaves the viewport) |
| EditorScrollToCenter | Ctrl+M | Scroll to Center |
| EditorToggleInsertState | Insert | Toggle Insert/Overwrite |
| EditorToggleColumnMode | Alt+Shift+Insert | Column Selection Mode |
| SelectNextOccurrence | Alt+J | Add Selection for Next Occurrence |
| UnselectPreviousOccurrence | Alt+Shift+J | Unselect Occurrence |
| SelectAllOccurrences | Ctrl+Alt+Shift+J | Select All Occurrences |
| EditorAddCaretPerSelectedLine | Alt+Shift+G | Add Carets to Ends of Selected Lines |
| EditorEscape | Escape | collapses carets and selection (section 8.5) |
| EditorAddOrRemoveCaret | Alt+Shift+Click | Add or Remove Caret |
| EditorCreateRectangularSelectionOnMouseDrag | Alt+Drag, middle-button drag | Create Rectangular Selection on Mouse Drag |
| EditorAddRectangularSelectionOnMouseDrag | Ctrl+Alt+Shift+Drag | Add Rectangular Selection on Mouse Drag |
| EditorCreateRectangularSelection | Alt+Shift+middle click | Create Rectangular Selection |
| EditorCloneCaretAbove / EditorCloneCaretBelow | no default key (double Ctrl, then Up or Down, when enabled) | Clone Caret Above / Below |
| GotoLine | Ctrl+G | Go to Line:Column |

`ED-CARET-001` and later requirements give the behavior; the keymap itself is data and is specified
by the keymap work, not here.

## 3. Words, camel humps and caret stops

Word movement needs a definition of "word boundary" at an offset, a way to treat camel humps, and
a policy that says where a move stops.

### 3.1 Word and hump boundaries (text based)

The scanner works on the text, with the help of the highlighter's token stream (for quoted strings
and token kind changes). A character is a *word character* if it is a letter, digit, underscore or
a dollar sign (the Java identifier-part test). A *punctuation* character is neither a word
character nor whitespace.

A **word start** is an offset where the character after the offset is a word character and the one
before is not, or where the character after is punctuation and the one before is not. A **word
end** is the mirror image (character before is a word character or punctuation and the one after
is not of the same class). In **camel mode** an additional boundary ("hump") exists inside an
identifier:

1. between a lower-case letter or digit and a following upper-case letter (`fooBar`: before `B`);
2. between two upper-case letters when the second is followed by a lower-case letter (`HTMLParser`: before `P`);
3. around an underscore: after `_` when followed by a non-underscore, and before `_` when preceded by a non-underscore (so `foo_bar` has humps before and after the underscore);
4. after a `$` that is followed by a letter or digit.

A **lexeme boundary** is an additional word stop between two adjacent tokens of the highlighter
when the language says they are different "words" (`foo.bar` or `a+b` where the language
word-boundary filter agrees) and the offset is not between two whitespace characters. A quoted
string token (opening and closing quote of the same kind, `"` or `'`, length at least 2) adds stops
just inside both quotes, so Ctrl+Right at the start of `"word"` goes inside the quote, then to the
end of the word, then past the closing quote.

### 3.2 Caret stop policy

A **caret stop policy** has two halves: the *word* half and the *line* half, each for backward and
forward movement; each half is one of `NONE`, `START`, `END`, `BOTH` (where to stop: at word/line
starts, at word/line ends, at both).

| Setting label (ApplicationBundle) | Word half backward / forward |
|---|---|
| Jump to the current word boundaries | START / END |
| Always jump to the word start | START / START |
| Always jump to the word end | END / END |
| Jump to the next/previous word boundaries | END / START |
| Stop at both word boundaries | BOTH / BOTH |

| Setting label (line half) | Backward / forward |
|---|---|
| Jump to the next/previous line boundaries | END / START |
| Ignore line breaks | NONE / NONE |
| Stop at both line boundaries | BOTH / BOTH |
| Jump to the current line boundaries | START / END |
| Always jump to the line start | START / START |
| Always jump to the line end | END / END |

The group is `Caret Movement` with the two combo boxes `When moving by words:` and `Upon line break:`
under Settings, Editor, General. The setting is stored **per operating system**. The IDE default
(used on Linux and by the kit) is `Jump to the current word boundaries` with `Jump to the next/previous
line boundaries`. The Windows default is `Always jump to the word start` with `Stop at both line
boundaries`; the Unix default is `Jump to the current word boundaries` with `Ignore line breaks`.

`ED-CARET-001` and `ED-CARET-010` below use these policies.

## 4. Smart Home, smart End, Page keys

### 4.1 Home

When the caret is on the first visual line of a logical line:

* if `Home moves caret to first non-whitespace character` (smart Home) is **on** (default): when the
  caret is at column 0 it moves to the first non-blank character of the line (if the line is blank
  it moves to the indent of the closest preceding non-blank visual line); when the caret is anywhere
  else it moves to the first non-blank character, **unless** it is already at or before that
  position, in which case it moves to column 0. So repeated Home toggles between first non-blank
  and column 0;
* if smart Home is off: always column 0.

If the caret is on a soft-wrapped continuation line, Home goes to the start of that visual line
(past the wrap indent first when smart Home is on, then to column 0 of the visual line, then to
the logical line start by the same toggling rules). Lines hidden inside a collapsed fold count as
part of the logical line the caret is on: Home goes to the start of the visual line that begins the
fold.

### 4.2 End

End goes to the end of the visual line. With trailing blanks, the first press stops after the last
non-blank character; a second press (caret already there) moves to the true end of the line. If
the visual line ends at a soft wrap, End first goes to the end of that visual line, then to the end
of the logical line. The setting `End on blank line moves caret to indent position` (default: see
`ED-CARET-100`) changes End on a whitespace-only line to go to the indentation column.

### 4.3 Page Up, Page Down, page top and bottom

Page Up and Page Down scroll the viewport by the number of whole lines that fit in it, and move
the caret by the same number of visual lines (keeping the column, or the remembered horizontal
position). The scroll is clamped so the last page does not scroll past the end of content (plus the
optional extra page, see `ED-APPEAR-` settings). Move Caret to Page Top/Bottom moves the caret
within the current viewport to the first/last fully visible visual line without scrolling.

## 5. Requirements: single caret movement

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-CARET-001 | MUST | Left and Right move the caret by one code point (a surrogate pair is one step; a combining sequence is not special). At the document start Left does nothing; at the end Right does nothing. A collapsed fold region (see file 03) counts as one step; moving into it lands at its edge. | A document with an emoji: Right moves over both UTF-16 units at once; at offset 0 Left leaves the caret at 0. |
| ED-CARET-002 | MUST | If a selection exists and the caret is within its bounds, Left collapses to the **start** of the selection and Right collapses to the **end** of it, without moving further. Sticky selection (ED-CARET-052) disables this rule: the move extends instead. | Select `abc` forward, press Left: caret at the start of `abc`, no selection. Press Right: caret at the end. |
| ED-CARET-003 | MUST | Up and Down move by one visual line, keeping a remembered horizontal position (so moving through a short line returns to the old column on the next longer line). If a selection exists (and sticky selection is off), the caret first jumps to the selection start (Up) or end (Down), then moves one line. In one-line editors Up and Down do nothing. | Line lengths 10, 2, 10 with the caret at column 8: Down, Down returns to column 8. With a selection, Down starts from the selection end. |
| ED-CARET-004 | MUST | Left, Right, Up and Down at the document edge do not wrap. Up on the first line and Down on the last line leave the caret where it is (they do not jump to line start or end). | Up on line 0: caret unchanged. |
| ED-CARET-005 | MUST | Moving the caret scrolls the primary caret into view with the minimal scroll that respects the vertical scroll offset (default 1 line) and horizontal scroll offset (default 3 columns) of the Scroll Offset settings (`ED-APPEAR-` settings). Secondary carets do not scroll. | Down from the last visible line scrolls by one line; with five carets only the primary scrolls. |
| ED-CARET-006 | MUST | Every move with selection (Shift variant) keeps the lead selection offset and moves the other end to the new caret offset, so selection can shrink through zero and then grow in the other direction. | Select `abc` forward from offset 3 to 6; Shift+Left four times: selection ends as offsets 2..3 (the caret is left of the anchor). |
| ED-CARET-007 | MUST | Caret moves do not modify the document, are not undoable commands by themselves, but are recorded for navigation history (see ED-NAV-) when they cross a distance. | Undo after only caret moves does nothing to the text (see ED-UNDO-). |
| ED-CARET-008 | SHOULD | At an offset that is a boundary between a left-to-right and a right-to-left text run, the caret may be moved without changing the offset by changing the side it leans toward; with selection, the same applies. | Bidirectional text sample: pressing Right at the run boundary first changes lean, then moves. |

## 6. Requirements: word, line and text boundaries

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-CARET-010 | MUST | Next Word and Previous Word move to the next stop defined by the caret stop policy of section 3.2: the word half decides stops inside a line, the line half decides what happens at line ends. With the IDE default, Next Word from the middle of `foo` goes to the end of `foo`; Previous Word from the middle goes to its start. | Text `foo bar`, caret in `foo`: Ctrl+Right lands after `foo`; Ctrl+Left from the middle of `bar` lands before `bar`. |
| ED-CARET-011 | MUST | The line half applies as: with a forward stop of `START`, Next Word never goes beyond the start of the next line; with `END`, never beyond the end of the current line (or, when already at the end, the end of the next line); with `NONE` it ignores line breaks entirely. Backward is symmetric. At the document ends the result is the document end (forward) or offset 0 (backward). | Default policy: at the end of line 1, Ctrl+Right puts the caret at column 0 of line 2; Ctrl+Left at column 0 of line 2 puts it at the end of line 1. |
| ED-CARET-012 | MUST | Camel humps: the setting `Use "CamelHumps" words` (default **off**) makes Next/Previous Word, word deletion and double-click selection stop at hump boundaries (section 3.1). The two actions `Move Caret to Next/Previous Word in Different "CamelHumps" Mode` (and their selection and deletion variants) do the opposite of the current setting for one invocation. They have no default shortcut. | With the setting on, Ctrl+Right in `fooBarBaz` stops after `foo`, then `Bar`, then `Baz`; with it off, one stop at the end. |
| ED-CARET-013 | MUST | If the caret is inside or at the edge of a collapsed fold region, a word move continues from the region edge: Next Word jumps to the region end; Previous Word to its start. A target that falls inside a region is moved to the region edge. | Caret before a collapsed `{...}`: Ctrl+Right lands after it. |
| ED-CARET-014 | MUST | At a position where the caret is at a text direction boundary, Next Word does not jump over the boundary: the target is limited to the nearest direction boundary in the move direction. | See ED-CARET-008 data. |
| ED-CARET-015 | MUST | Home and End follow section 4 including the toggling of smart Home and the trailing blank handling of End; Shift variants keep the anchor. With smart Home off, Home always goes to column 0; End never ignores trailing blanks when `ignore trailing whitespace` does not apply (Select-to-end uses the same rule). | Line `    foo   ` with the caret mid-word: End goes after `foo`, a second End to the true line end; Home goes to column 4, a second Home to column 0, a third to column 4. |
| ED-CARET-016 | MUST | Move Caret to Text Start / End go to offset 0 / document end, remove all secondary carets and the selection, scroll (End scrolls to center), and record the move for navigation history. The Shift variants extend the selection from the anchor and keep other carets. | Five carets, Ctrl+Home: one caret at 0, no selection. |
| ED-CARET-017 | MUST | Page Up/Down, Move to Page Top/Bottom follow section 4.3; Page keys keep the column; with Shift they extend the selection. At the last page Page Down moves the caret as far as the content allows and scrolls no further than the content end (plus an extra page when `Show virtual space at the bottom of the file`, default off; see file 07). | A 100-line file in a 30-line viewport: three Page Down presses end at the last line with the viewport not past the end. |
| ED-CARET-018 | SHOULD | Scroll Up/Down (Ctrl+Up/Down) scroll by one line without moving the caret unless it would leave the viewport, in which case it moves just enough to stay visible. Scroll to Center centers the caret line. Actions `Scroll to Top/Bottom`, `Scroll Left/Right`, `Move Up/Down and Scroll` exist without default shortcuts. | Ctrl+Down scrolls one line; the caret line is unchanged until it reaches the top edge. |
| ED-CARET-019 | SHOULD | `Move Caret to Line Start/End` and Next/Previous Word accept being invoked for each caret independently when several carets exist; each caret moves by its own rule and carets that become equal are merged (primary stays primary). | Two carets adjacent after Ctrl+Right merge into one caret. |
| ED-CARET-020 | MUST | Toggle Insert/Overwrite (Insert key) flips between insert mode (default) and overwrite mode. In overwrite mode a typed character replaces the character after the caret (not a line break), the caret is drawn as a block, and the status bar shows the mode. The mode is per editor and is not persisted. | Insert key, type `x` before `a`: `a` is replaced by `x`. |

## 7. Requirements: selection commands

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-CARET-030 | MUST | Select All (Ctrl+A) selects the whole document with the caret at the end, removes secondary carets, and does not scroll. | One caret at EOF, selection 0..len. |
| ED-CARET-031 | MUST | **Extend Selection** (Ctrl+W) selects the next larger syntactic range that contains the current selection and is not equal to it. The ranges come from registered *selection extenders*, queried for the leaf at the caret and each ancestor (and for every language tree of a multi-language file); the result is the smallest candidate range that contains the current selection strictly. Candidate ranges are in order: the word (camel hump first when camel mode is on, then the whole word), then language ranges (string literal contents, comment text, argument, argument list, statement, block body without braces, block with braces, member, class body, class, file). If nothing contains the selection the whole file is selected. Repeated presses grow step by step. | In `foo(bar, baz)` with the caret in `bar`: `bar`, then `bar, baz`, then `(bar, baz)`... until the whole file, one range per press. |
| ED-CARET-032 | MUST | Extend Selection adjusts the starting offset first: if the caret is just after a word character and before a non-word character (or at the end of text or before whitespace), the character before the caret is used as the starting point so that a caret at the end of a word selects that word. | `foo|` selects `foo` on the first press. |
| ED-CARET-033 | MUST | If the caret is in whitespace, the nearest non-whitespace element to its left is used when the whitespace cannot itself be expanded; otherwise the following element is used (whitespace that spans several lines extends to a range of blank lines first). | Caret in the indentation of a line extends to the statement on that line. |
| ED-CARET-034 | MUST | Extend Selection never selects a range that cuts a collapsed fold region; ranges are widened to the region edges. It works per caret. | Selection range touching a collapsed fold includes the whole fold. |
| ED-CARET-035 | MUST | Extend Selection commits pending changes to the syntax tree first and runs with a cancellable progress titled `Selecting word at given offset...`; if it is cancelled or fails the selection is unchanged. In an injected language fragment it uses the injected range first and the host when the selection already equals the fragment's editable content. | Cancelling leaves the selection as it was. |
| ED-CARET-036 | MUST | **Shrink Selection** (Ctrl+Shift+W) goes back one step: it picks the largest candidate range (same candidate set as ED-CARET-031) that is contained in the current selection, differs from it, and contains the caret (or ends at it). If none exists it collapses the selection to the caret. | Extend three times then Shrink once returns to the second range. |
| ED-CARET-037 | SHOULD | Extension chains are *stable*: Extend after Shrink reproduces the same sequence, and the sequence for a given caret position does not depend on how the caret got there. | Extend, Shrink, Extend returns the same range as the first Extend. |
| ED-CARET-038 | MUST | `Select Single Line at Caret` selects the logical line (with its line break) and moves the caret to the line start; `Extend Line Selection` (no default key) selects whole lines covering the current selection, and each repeat adds one more line. | Repeated invocation grows the selection by one line each time. |
| ED-CARET-039 | MUST | A word is selected by double click (using camel humps only when `Honor "CamelHumps" words settings when selecting on double click`, default **on**, and camel mode is on) and the line by triple click. Click and drag extend by character; after a double click drag extends by word, after a triple click by line. | Double click on `fooBar` with camel off selects `fooBar`. |
| ED-CARET-040 | SHOULD | `Swap Selection Boundaries` moves the caret to the other end of the selection (selection unchanged); with sticky selection on, the anchor is re-set. No default key. | Caret at the end, swap: caret at start, same range. |
| ED-CARET-041 | MUST | Typing, deleting, or any command that is not a pure caret move removes the selection or replaces it as that command specifies; the selection is never silently kept across edits. | Typing over a selection replaces it (file 02). |
| ED-CARET-042 | SHOULD | Selection is copied to the **primary selection** on systems that have one (X11) when `editor.caret.update.primary.selection` (default on) is set; middle click pastes it (`Paste from X Clipboard`, mouse button 2). | On Linux, selecting text then middle-clicking in another editor pastes it. |

## 8. Requirements: code block, brace, sticky selection, column mode, multiple carets

### 8.1 Code block and brace

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-CARET-050 | MUST | Move Caret to Code Block End/Start (Ctrl+] and Ctrl+[) jumps to the closing/opening structural brace of the innermost enclosing block (language-provided block range first; else the indent guide of the caret line; else brace matching on structural braces counted by depth, restricted to one language). Repeating moves to the next enclosing block. The Shift variants keep the anchor. The jump is recorded in navigation history. | Inside nested `{}` Ctrl+] goes after the inner `}` then the outer one. |
| ED-CARET-051 | MUST | Move Caret to Matching Brace (Ctrl+Shift+M) jumps from a brace to its partner, using the brace matcher; when the caret is not at a brace it finds the closest enclosing structure: the language block range's start or end (whichever is farther from the caret's current token), or the nearest unclosed opening brace before the caret. Selection is removed; it never fails silently when a brace exists nearby. | Caret after `)` jumps to before the matching `(`. |
| ED-CARET-052 | MUST | **Sticky selection** (action `Toggle Sticky Selection`, no default key; also entered by a mouse gesture of the editor): while on, movement commands extend the selection; a command that copies or cuts, and Escape, turn it off. | Toggle on, press Right three times: three characters selected. |

### 8.2 Column (block) selection

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-CARET-060 | MUST | **Column Selection Mode** (Alt+Shift+Insert) is a toggle. Turning it on removes secondary carets and turns an existing selection into a block selection with the same corners; turning it off converts the block into one multi-caret selection per line and then merges them to one selection from the first anchor to the last caret end. In column mode the caret can be placed past the end of lines (virtual space). | A 3-line selection turns into a 3-line rectangle; leaving the mode keeps the text selected. |
| ED-CARET-061 | MUST | Alt+Drag (or middle-button drag) creates a rectangular selection regardless of the mode; Alt+Shift+Click or the Alt+Shift+middle-click gesture creates one between the previous click point and the new one; Ctrl+Alt+Shift+Drag adds a further rectangle without clearing existing carets. Each line of a block has its own caret and selection, ordered by line. Lines shorter than the block's left edge get a caret at their end with an empty selection (virtual space used only in column mode). | Dragging with Alt over four lines yields four carets. |
| ED-CARET-062 | MUST | In column mode, Shift+Arrow resizes the block, typing and deleting apply to every line, and a column-mode paste of N lines into N carets pastes one line per caret (see file 03, clipboard). | Four-line block, type `x`: four `x` inserted at the same column. |
| ED-CARET-063 | SHOULD | `Add Carets to Ends of Selected Lines` (Alt+Shift+G) removes each selection and places one caret at the end of every line it covered (a selection ending exactly at a line start does not include that last line). The primary caret is the one at the end of the line where the original caret was, when the original caret was at the selection's end side. Fails with the hint below when the maximum caret count would be exceeded. | A three-line selection becomes three carets at line ends. |

### 8.3 Multiple carets

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-CARET-070 | MUST | The editor supports multiple carets, each with its own selection. All editing commands run once per caret, in a consistent order (from the end of the document to the start for text insertions so earlier offsets stay valid; the visible result equals a simultaneous edit). Carets that would overlap or touch an identical position after a command are merged; a merged group keeps the primary caret role when it contained the primary. | Two carets, type `ab`: both get `ab`. Two carets that Left-move to the same offset become one. |
| ED-CARET-071 | MUST | The maximum number of carets is 1000 (registry `editor.max.caret.count`, default 1000, never below 1). Creating one more shows the hint `Cannot create more than 1000 carets` (message `Cannot create more than {0} {0,choice,1#caret\|1<carets}`) and does nothing; restoring a larger saved caret list truncates to the limit with the same hint. | Select all occurrences of `a` in a file with 1500 occurrences: 1000 carets and the hint. |
| ED-CARET-072 | MUST | Alt+Shift+Click adds a caret at the click point, or removes the existing caret there (the last remaining caret is not removable). | Click twice on the same spot returns to one caret. |
| ED-CARET-073 | MUST | **Clone Caret Above / Below** adds a caret in the same visual column of the previous/next line (using the remembered horizontal position, not the column reached by a recent move); a caret with a selection is cloned with an equal-length selection on that line when the line is long enough, otherwise on the nearest line that can hold it. Repeated invocation in the same direction extends the column of carets; invoking in the opposite direction after cloning removes the farthest ones first (a signed "level" is tracked per caret group; the tracking is reset when any other action runs). The action is also bound to double Ctrl followed by Up/Down when `Add multiple carets on double Ctrl with arrow keys` (default **on**) is on; the double-press variants of Shift+Arrow and the line/page/word moves with selection are part of the same repeat group. | Caret at line 5: Clone Below twice gives carets on lines 5, 6, 7; Clone Above then removes line 7 and 6... |
| ED-CARET-074 | MUST | **Add Selection for Next Occurrence** (Alt+J): without a selection it selects the word at the caret (word characters = identifier characters; whole-word matching is switched on for the repeat series). With a selection it adds a new caret with a selection on the next match, searching forward from the end of the primary selection, case sensitive; whole-word matching stays on only when the series started from an empty selection. The match is selected with the same caret-to-selection offset as the original. The viewport scrolls to the new caret. | `foo foo foo`, caret in the first `foo`: three Alt+J give three selections. |
| ED-CARET-075 | MUST | When no further match exists the hint `Not found, try again to search from top` is shown (under the caret, hidden by text change or scrolling) and the next consecutive Alt+J restarts the search from the document start; matches already selected are skipped (no duplicate carets). If all matches are already selected nothing changes. A collapsed fold region that contains a match is expanded (unless it can never expand). | Last occurrence reached: hint; next press wraps to the first unselected one. |
| ED-CARET-076 | MUST | **Unselect Occurrence** (Alt+Shift+J) removes the primary caret (the latest added one) when there are several carets; with only one caret it removes the selection. The match-search state is reset. | After three Alt+J, one Alt+Shift+J leaves two selections. |
| ED-CARET-077 | MUST | **Select All Occurrences** (Ctrl+Alt+Shift+J): with no selection, selects the word at the caret (whole-word matching on); then selects every match in the document, case sensitive, one caret per match, the primary being the one that corresponds to the original caret. Requires a project and multi-caret support; disabled when the search is disabled for the editor. | `a b a b a`: three carets after selecting `a`. |
| ED-CARET-078 | MUST | The **select-next-occurrence state** (not-found flag, whole-word flag) survives only while the last performed action is one of Add Selection for Next Occurrence, Unselect Occurrence, Find Next or Find Previous; any other action resets it. | Press Alt+J, type, press Alt+J: whole-word matching is reset. |
| ED-CARET-079 | SHOULD | Caret positions are tracked through document changes by the document, not recomputed by the editor: after an edit elsewhere each caret keeps its logical meaning (offset adjusted by range-marker rules, selection ends greedy to neither side). | Insert text above a caret: the caret stays on the same character. |
| ED-CARET-080 | SHOULD | The set of carets and selections can be saved and restored as a state list (used by undo, by copy of empty selection, and by editor state restore on reopening a file); a restored list longer than the maximum is truncated. | Close and reopen a file: caret and selections are back (see ED-TABS-). |

### 8.4 Escape

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-CARET-090 | MUST | Escape (when the editor has focus, no popup or lookup or live template consumes it first) turns off sticky selection; then, if there are several carets, **keeps only the oldest caret** (removes the primary caret repeatedly, so the caret created first survives), removes the selection, and scrolls the survivor into view. The action is enabled only when there is a selection or more than one caret; otherwise Escape is passed on (for example to focus handling: `FocusEditor` and `HideActiveWindow` take it). In a one-line editor inside a modal dialog it is disabled. | Three carets, Escape: one caret (the first), no selection. One caret with a selection, Escape: selection removed, caret stays. No selection, one caret: Escape does nothing in the editor. |
| ED-CARET-091 | MUST | The order in which Escape is consumed is: lookup popup, live template or in-place refactoring, parameter-info and hint popups, then the caret and selection rule above, then the tool window or focus rule. | With the completion popup open, Escape closes only the popup. |

### 8.5 Settings that affect this area

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-CARET-100 | MUST | Defaults: `Home moves caret to first non-whitespace character` on; `End on blank line moves caret to indent position` on (the code insight setting `SMART_END_ACTION`); `Use "CamelHumps" words` off; `Honor "CamelHumps" words settings when selecting on double click` on; `Add multiple carets on double Ctrl with arrow keys` on; `Highlight occurrences of selected text` on (suppressed when more than 50 occurrences, registry `editor.highlight.selected.text.max.occurrences.threshold`); caret blinking on with a period of 500 ms (range 10 to 1500); block caret off; full-line-height caret off; smooth caret movement off; caret width 2 px (registry). Where the settings page puts them is in file ED-APPEAR (06). | Fresh profile: each control shows the default. |
| ED-CARET-101 | SHOULD | `Allow caret placement: After the end of line` (virtual space, default **off**) and `Inside tabs` (default **on**, caret may rest inside a tab's width) are honored by click, Up/Down and column mode. Without virtual space a click past the end of the line places the caret at the line end. | Virtual space off: click in empty area right of a line puts the caret at the line end. |
| ED-CARET-102 | LATER | Caret animation: smooth caret movement (duration 120 ms, easing `Snappy` default) and blinking styles other than the default. | Setting on: caret glides between positions. |

## 9. Interaction with folding and other models

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-CARET-110 | MUST | A caret is never placed strictly inside a collapsed fold region by user commands: movement skips to the edge; a click inside moves to the nearest edge (or expands, see file 03). Programmatic moves into a region expand it unless the region must never expand. | Press Right before a collapsed region: caret after it. |
| ED-CARET-111 | SHOULD | Regions that are selectable as a unit (for example a collapsed region marked to be selected when the caret is next to it) are selected as a whole when the caret moves onto them with Shift or when it lands next to them. | Moving the caret next to such a region with Shift selects the region. |
| ED-CARET-112 | MUST | Inlay hints and other inlays occupy visual columns but no offsets: Left/Right move over an inlay in one step only when the caret has a distinct visual position next to it (before-and-after the inlay), never deleting or entering it. Clone Caret treats the inlay as zero-width on the target line. | Caret before an inlay: Right passes it with the caret ending after it in a distinct visual position. |

## 10. Open questions

* Exact set of "selection extender" ranges of the Java and Kotlin languages (their order for strings,
  comments, argument lists, generics, chained calls); to be specified with the language, not here.
* The default shortcut of Clone Caret above/below in the Windows keymap: none is registered in the
  default keymap; the double-Ctrl gesture is the only built-in default. Confirm with a running IDE.
* The `editor.caret.update.primary.selection` behavior on Wayland.
* Whether `End on blank line moves caret to indent position` default is on in all products
  (source: on in the shared settings class).
* Exact algorithm of the remembered horizontal position across soft wraps and inlays.

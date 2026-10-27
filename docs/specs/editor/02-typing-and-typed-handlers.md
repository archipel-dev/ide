# Editor spec 02: typing, Enter, Backspace, Tab, pairs, paste, Smart Enter

Status: specification, language independent unless a row says Java. Requirement prefix `ED-TYPE`.
Priorities: `MUST` / `SHOULD` / `LATER`.

This file specifies what the editor does when the user types a character, presses Enter,
Backspace, Delete, Tab or Shift+Tab, pastes, or invokes Complete Current Statement (Smart Enter).
Line-oriented commands (duplicate, delete line, join, move, comment, case) and copy/cut are in
`03-line-and-text-operations.md`. Defaults of the settings used here are collected in section 9 and
shared with `codeinsight/08-optimize-imports-and-reformat.md` section 7 (`CI-OPT-060`), which lists
the same hooks at a high level; this file is the detailed contract and wins on details.

Executable acceptance cases (before text, keys, after text) are in
`cases/02-typing-and-typed-handlers.cases.md`; the format is in `cases/README.md`. Cases are
normative; a requirement row that disagrees with a case is a defect.

ic: `platform/lang-impl/src/com/intellij/codeInsight/editorActions/` (`TypedHandler.java`,
`TypedCharImpl.java`, `TypedParenImpl.java`, `TypedQuoteImpl.java`, `SelectionQuotingTypedHandler.java`,
`TabOutScopesTrackerImpl.java`, `BraceOrQuoteOutAction.java`, `TabActionsPromoter.java`,
`BackspaceHandler.java`, `AbstractIndentingBackspaceHandler.java`, `SmartIndentingBackspaceHandler.java`,
`SimpleIndentingBackspaceHandler.java`, `BackspaceToWordStartHandler.java`, `EnterHandler.java`,
`PasteHandler.java`, `enter/*.java`, `smartEnter/SmartEnterAction.java`),
`platform/platform-impl/src/com/intellij/openapi/editor/actions/` (`TabAction.java`,
`IndentSelectionAction.java`, `UnindentSelectionAction.java`, `HungryBackspaceAction.java`,
`DeleteToWordBoundaryHandler.java`, `BackspaceAction.java`, `SplitLineAction.java`,
`StartNewLineAction.java`, `StartNewLineBeforeAction.java`),
`platform/analysis-impl/src/com/intellij/codeInsight/CodeInsightSettings.java`,
`java/java-frontback-impl/src/com/intellij/codeInsight/editorActions/` (`JavaBackspaceHandler.java`,
`JavaQuoteHandler.java`, `smartEnter/JavaSmartEnterProcessor.java`),
`platform/ide-core/resources/messages/ApplicationBundle.properties`,
`platform/platform-api/resources/messages/EditorBundle.properties`,
`platform/platform-resources/src/keymaps/$default.xml`.

## 1. The typing pipeline

For one typed character the editor runs, **once per caret** (carets are processed in a fixed order,
the primary first for the auto-popup step), these steps. A step that reports "handled" stops the
rest for that caret.

1. The document must be writable; otherwise nothing is inserted, the read-only hint
   `This view is read-only` is shown for viewers, and the guarded-block message
   `Unable to perform an action since it changes read-only fragments of the current document`
   (title `Guarded Block Modification Attempt`) for guarded fragments.
2. If the caret is inside an injected-language fragment and the typed character is significant for
   it, the injected editor is used for the rest.
3. Auto-popup: for the primary caret only, the completion auto-popup and the parameter-info popup
   are considered (see `06-completion`, `CI-CMP`, and section 9 for delays).
4. In **overwrite mode** the character replaces the next character (never a line break) and the
   pipeline ends.
5. "Before selection removed" hooks run (surround selection, quote replacement: section 3.4).
6. The selection is deleted (the typed character replaces it).
7. "Before char typed" hooks run (language typed handlers, may handle the character).
8. Closing-bracket overtype (3.2) and closing-quote overtype / quote pairing (3.3).
9. The character is inserted. The command is named `Typing`, and the document is registered as
   affected by the command (undo grouping: `04-undo-redo`).
10. Auto hard wrap at the right margin, if enabled (`Wrap when typing reaches right margin`,
    default off).
11. "After bracket typed": auto-insert of the closing bracket (3.1), reformat of a typed `}` or `)`.
12. "Char typed" hooks run (language handlers: for example `>` after a tag name).
13. Indent of a typed `{` or `(` that starts a line.

The next sections give the contract of each step.

## 2. Plain typing

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-TYPE-001 | MUST | A typed character is inserted at every caret; selections are replaced (deleted first), so the result at each caret is the typed text with the caret after it. Typing in a multi-caret editor equals typing at each caret independently. | Cases ED-TYPE-001. |
| ED-TYPE-002 | MUST | Typing never throws or panics when the file has no syntax support, when the project is not loaded, or when a language handler fails; the character is inserted by the plain rule. | A plain file with no language handlers: typing works. |
| ED-TYPE-003 | MUST | In overwrite mode a typed character replaces the next character on the line; at the end of a line it inserts. With a selection it replaces the selection as in insert mode. | Case with Insert key (see also `ED-CARET-020`). |
| ED-TYPE-004 | MUST | Typed characters that are surrogates are inserted as one code point; typing a high surrogate followed by a low surrogate yields the pair. | Type U+1F600: one pair, caret after both units. |
| ED-TYPE-005 | MUST | The typed character is inserted through the **typing command** named `Typing`; the command merges with following typing for undo (`ED-UNDO-`). | Type `abc`, Ctrl+Z once: all three gone. |
| ED-TYPE-006 | SHOULD | Typing in a read-only editor shows the hint `This view is read-only` (viewer) and does not change text. Typing in a non-writable file attempts to make it writable through the version control/file system prompt (out of scope here: the editor only asks "is modification allowed"). | Read-only document: typing leaves text unchanged. |

## 3. Brackets, quotes and tab-out

### 3.1 Auto-insert closing bracket

Default settings: `Insert paired brackets (), [], {}, <>` on; `Insert pair quote` on; `Surround selection
on typing quote or brace` on; `Jump outside closing bracket/quote with Tab when typing` on. The
rules need the language's **brace matcher** (pairs, which are structural) and the highlighter
tokens; plain text files never pair brackets.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-TYPE-010 | MUST | After typing `(`, `[` or `{` in a non-plain-text file, when the setting is on, the matching closer is inserted after the caret **unless** (a) the next token after the caret is of a kind before which paired braces are not allowed in that file type (for the Java family: pairs are allowed only before whitespace, a closing bracket or brace, a semicolon, a comma, an end of file, or a comment; not before an identifier, a literal or an opening bracket), or (b) the new opening bracket already has a partner in the file (counting from the leftmost unmatched opener of the same kind, the structure is already balanced). The caret stays between the pair. | `foo<caret>` then `(` gives `foo(<caret>)`; `foo<caret>bar` then `(` gives `foo(<caret>bar`. |
| ED-TYPE-011 | MUST | The closer inserted by 3.1 registers a **tab-out scope**: a tracked empty range between the brackets with the position after the closer as exit. The scope is per caret and survives typing inside it, and ends when the caret leaves it or the document changes in a way that destroys it. | See ED-TYPE-030. |
| ED-TYPE-012 | MUST | In Java and Kotlin, typing `<` after a class-like identifier or after a dot (`List`, `Map.Entry`) where generics are available inserts `>` after the caret; backspacing the `<` of such a pair removes the `>` too. Typing `<` in an expression (after a number, a closing parenthesis, whitespace) does not. | `List<caret> x` then `<` gives `List<<caret>> x` in a Java declaration. |
| ED-TYPE-013 | MUST | If the setting `Insert paired brackets` is off, no closer is inserted and the overtype rule 3.2 does not apply. | Setting off: `(` inserts only `(`. |
| ED-TYPE-014 | MUST | When a typed `{` or `(` is the first non-blank character of its line, the line is re-indented to the code style position (the new text and only that line); when a `}` or `)` is typed as the first character on its line it is re-indented too, and for a structural `}` with a matching `{` the whole block between them is **reformatted** when `REFORMAT_BLOCK_ON_RBRACE` (a code insight setting, default **on**) is on, else only the line is re-indented. The caret ends after the typed character and the selection is removed. | A `}` typed alone on a line with a wrong indent moves to the indent of its `{` line. |

### 3.2 Overtype of a closing bracket

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-TYPE-020 | MUST | Typing `)`, `]` or `}` when the next character is the same closer, in a non-plain-text file, with `Insert paired brackets` on, and when the next token is exactly that one-character closing brace token whose opener (found by scanning back for the leftmost unmatched opener of that kind) exists and matches, **moves the caret over the existing closer** instead of inserting. If no matching opener exists the character is inserted normally. | `foo(<caret>)` then `)` gives `foo()<caret>`; with no opener before it, `x<caret>)` then `)` inserts a second `)`. |
| ED-TYPE-021 | MUST | Overtype works for any closer, not only for those typed via ED-TYPE-010: `foo(a<caret>)` then `)` skips the existing one. | Case. |

### 3.3 Quotes

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-TYPE-025 | MUST | Quote characters are `"`, `'` and the back tick. With `Insert pair quote` on and a quote handler for the file type: typing a quote when the next character is the same quote **and** that character is the closing quote of a literal moves over it. | `"abc<caret>"` then `"` gives `"abc"<caret>`. |
| ED-TYPE-026 | MUST | A typed quote that is escaped (preceded by an odd number of backslashes inside a literal) is inserted as a plain character. | `"a\<caret>"`: typing `"` gives `"a\"<caret>"`. |
| ED-TYPE-027 | MUST | A typed quote at a position where a literal may start (token at the caret is whitespace, semicolon, comma, closing bracket or EOF for Java-like languages) inserts the quote and, if the new quote opens a literal that is not closed on its line and the character after the caret is not an identifier part, inserts the closing quote after the caret and registers a tab-out scope. If the next character is an identifier part only the opening quote is inserted. Inside an existing string literal or in a comment where a literal cannot start, the quote is inserted alone. | `x = <caret>;` then `"` gives `x = "<caret>";`; `<caret>abc` then `"` gives `"<caret>abc`; `"ab<caret>c"` then `"` gives `"ab"<caret>c"`. |
| ED-TYPE-028 | SHOULD | Multi-character quotes (Java text blocks `"""`, Kotlin raw strings, Python triple quotes) are completed by the language when the third quote is typed: the closing sequence is inserted when the literal is not closed and the next character is not an identifier part. | Java: `String s = ""<caret>;` then `"` gives a text block opening with its closer (see `ED-TYPE-028` case). |
| ED-TYPE-029 | MUST | Backspace right after a typed opening quote whose closing quote follows immediately removes both (see ED-TYPE-045). | `"<caret>"` Backspace gives empty. |

### 3.4 Tab out and selection surrounding

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-TYPE-030 | MUST | **Tab out** (action `BraceOrQuoteOut`, key Tab): when a tab-out scope ends exactly at the caret, Tab moves the caret to the scope's exit position (after the closing bracket or quote) and removes the scope; it takes priority over inserting a tab, over indent, and over live template Tab, but not over an open completion lookup. It works only when `Jump outside closing bracket/quote with Tab when typing` (default on) is on. Without a scope Tab behaves as in 5. | Java: type `foo(`, then `a`, then Tab gives `foo(a)<caret>`. |
| ED-TYPE-031 | MUST | Tab priority when several Tab actions are enabled: BraceOrQuoteOut first, then (Tab, Next Parameter, Emacs-style indent) in one tier, then others. Inside a live template the template's Next Variable wins (`ED-TMPL-`). | Tab with a scope and a template active: template wins when its variable range contains the caret. |
| ED-TYPE-035 | MUST | **Surround on typing**: with a selection and setting `Surround selection on typing quote or brace` on, typing `(`, `[`, `{`, `<`, `"`, `'` or the back tick wraps the selection with the pair instead of replacing it. The selection stays on the same text, now one character inside, with the caret at the same end as before. If the selection already starts with a delimiter of the same family and ends with its partner (and the partner occurs nowhere else inside), the old delimiters are replaced (not nested). Typing `{` additionally reformats the surrounded range. Does not apply when the selection is empty or blank-only. | `<selection>abc</selection>` then `(` gives `(<selection>abc</selection>)`. |
| ED-TYPE-036 | MUST | Exception: when the selected text is one to three characters made only of `<`, `>`, `=`, `!` and the user types `<` or `>` outside comments and string literals, the selection is replaced as a comparison operator being edited (no surround). | `a <selection>==</selection> b` then `<` gives `a < b`. |
| ED-TYPE-037 | SHOULD | **Quote replacement**: if the selection is exactly one quote character and a different quote is typed, and that character is the opening or closing quote of a literal whose other end is the same quote, both ends are changed to the typed quote and the literal body is re-escaped (a typed quote inside the body gains a backslash, a now-unneeded backslash before the old quote is removed); skipped when the body contains quotes in a language without escapes. | `"a<caret>b"` selecting the first `"` and typing `'` gives `'ab'`. |

## 4. Backspace and Delete

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-TYPE-040 | MUST | Backspace with a selection deletes the selection. Without one it deletes the code point before the caret (a surrogate pair is deleted whole). At the start of a line it joins with the previous line (deletes one line separator) and puts the caret at the join point. At offset 0 it does nothing. | `ab<caret>c` gives `a<caret>c`; `ab` newline `<caret>cd` gives `ab<caret>cd`. |
| ED-TYPE-041 | MUST | A collapsed fold region that must never expand and ends at the caret is deleted entirely by Backspace; other collapsed regions are not entered. | See file 03. |
| ED-TYPE-042 | MUST | Backspace is applied once per caret and works on all carets in the same command. | Three carets, Backspace: one character removed at each. |
| ED-TYPE-043 | MUST | After a Backspace that deletes `(`, `[` or `{` and with `Insert paired brackets` on: if the character now after the caret is the matching closer and that closer's structure is balanced only by this pair (no other opener would match it), the closer is deleted too. Not done if the closer belongs to another opener, or when the deleted character was selection-replaced. | `foo(<caret>)` Backspace gives `foo<caret>`; `foo(<caret>)` where `)` has another `(` before it gives `foo<caret>)`. |
| ED-TYPE-044 | MUST | The same for `<` (Java generics) as in ED-TYPE-012. | See that case. |
| ED-TYPE-045 | MUST | After a Backspace that deletes an **opening** quote (`"`, `'`, back tick), with `Insert pair quote` on: if the next character is the same quote and that quote is an opening quote by the language's quote handler (an empty literal) it is deleted too; a closing quote is never deleted (so Backspace on `"abc"<caret>` deletes only the final quote). | `"<caret>"` gives empty; `"abc"<caret>` Backspace gives `"abc<caret>`. |
| ED-TYPE-046 | MUST | **Unindent on Backspace** (`Unindent on Backspace:`; values `Disabled`, `To nearest indent position`, `To proper indent position`; default **To proper indent position**; the language may override). It acts only when the character before the caret is a blank and everything between the line start and the caret is blanks. Mode *nearest*: the caret moves to the previous multiple of the indent size (columns), by deleting or, for a caret left of the target, inserting spaces; the first Backspace on column 3 with indent 4 goes to column 0. Mode *proper*: after the normal deletion, the whole leading whitespace of the line (through the whitespace after the caret) is replaced by the indent the code style computes for that line, with the caret after it; if the line's width is already at or below that proper indent, Backspace instead joins the line with the previous one (trailing blanks of the previous line are removed; the join space is chosen by the formatter, normally none before a line break and none at end of line) and puts the caret at the join point; on the first line it just clears the whitespace. Not in column mode. | Java: the empty auto-indented line below `int x;` then Backspace removes the whole line (caret at the end of `int x;`); `        int x;` with 8 spaces where 4 is proper and caret after the 8 spaces, Backspace gives 4 spaces and the caret before `int`. |
| ED-TYPE-047 | MUST | Hungry Backspace (action `EditorHungryBackSpace`, no default key): when the character before the caret is whitespace (including line breaks) all consecutive whitespace before the caret is deleted; otherwise it acts as Backspace. | `a   <caret>b` gives `a<caret>b`. |
| ED-TYPE-048 | MUST | Delete (key Delete) with a selection deletes it; otherwise deletes the code point after the caret; at the end of a line deletes the line separator (joining the next line with no whitespace adjustment); at the end of the document does nothing. | `ab<caret>` newline `cd` Delete gives `ab<caret>cd`. |
| ED-TYPE-049 | MUST | `Delete to Word Start` (Ctrl+Backspace) and `Delete to Word End` (Ctrl+Delete): with a selection delete the selection; otherwise delete from the caret to the previous word start (stop set: word starts and the line start) or to the next word boundary of either kind (stop set: word ends **and** starts, and the line end), honoring camel humps as in `ED-CARET-012`, and treating a quoted string's insides as in ED-CARET section 3.1. The "Different CamelHumps Mode" variants invert the camel setting. Deleting is a separate undo group from typing. | `foo bar<caret>` Ctrl+Backspace gives `foo <caret>`; `foo <caret>bar` gives `<caret>bar`; `<caret>foo bar` Ctrl+Delete gives `<caret> bar`. |
| ED-TYPE-050 | SHOULD | `Delete to Line Start`, `Delete to Line End`, `Cut up to Line End`, `Cut Line Backward` have no default key; they delete (and for Cut put on the clipboard) up to the line boundary; at a boundary they take the line separator: Line End deletes the separator when the caret is at the end or only blanks follow. | `ab<caret>cd` Delete to Line End gives `ab<caret>`. |

## 5. Enter

`EditorEnter` (Enter) inserts a line break and runs a chain of language-aware steps. The default
(plain) enter inserts `\n` at each caret after deleting the selection.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-TYPE-060 | MUST | The command is named `Typing` and is writable-only; selection is deleted first; every caret is handled. At the end the selection is removed and the caret is scrolled into view. | `a<selection>bc</selection>d` Enter gives `a` newline `<caret>d`. |
| ED-TYPE-061 | MUST | The **basic Enter** (used when no language step takes over) deletes the selection, then inserts a line break at the caret followed by a copy of the whitespace that starts the current line, limited to the part of that whitespace before the caret. This copying is the whole indent logic of plain text files and of `Smart indent` off. | Plain text: `    abc<caret>` Enter gives `    abc` newline `    <caret>`. |
| ED-TYPE-062 | MUST | **Smart indent** (`Smart indent`, default on): after the break, the new line's indent is recomputed by the language's line indent provider or formatter (the formatter's answer for an empty line at that position; for blocks this is one indent deeper after an opening brace and back out after a closing one). Languages without such logic keep the basic result. With the setting off only language handlers that force an indent (comment continuation, doc comments) change the basic result. | Java: after `void f() {` smart indent gives one level deeper than that line; with the setting off the new line only copies the leading whitespace of the line above. |
| ED-TYPE-063 | MUST | When the caret is inside the leading whitespace of a **non-blank** line, Enter performs only the basic Enter: the old line keeps the whitespace before the caret as trailing whitespace and the new line starts with the same whitespace followed by the text; no re-indentation happens. In overwrite mode Enter inserts nothing: it moves the caret to column 0 of the next line (appending a final line break first when on the last line). | `    <caret>foo` Enter gives a line of 4 spaces and then `    <caret>foo`; overwrite mode `a<caret>b` newline `cd` Enter gives the caret before `cd`. |
| ED-TYPE-064 | MUST | **Between braces** (`{` `}` or `(` `)` with the caret between them on the same line, braces in the same syntax element): Enter creates three lines: the opener line, an indented empty line holding the caret, and the closer on its own line with the opener line's indent. | Java: `void f() {<caret>}` gives the three-line form with the caret on the middle line, indented one level. |
| ED-TYPE-065 | MUST | **After an unmatched opening brace** (`Insert pair '}'`, default **on**; code insight setting `INSERT_BRACE_ON_ENTER`): in Java-like files, Enter after a `{` that has no matching `}` inserts the closing `}` on the line after the caret line, indented like the opener line, and puts the caret on the new empty indented middle line. | Java: `void f() {<caret>` (no closer) gives the three-line form. |
| ED-TYPE-066 | MUST | **Line comment continuation**: Enter in the **middle** of a line comment (text after the caret on the same line, ignoring whitespace) splits the comment: the new line starts with the same comment prefix and one space (or the same spacing the first line used after its prefix) followed by the rest of the text; whitespace before the rest is removed. At the **end** of a line comment (nothing but blanks after the caret) a plain Enter happens with no prefix. | Java: `// foo<caret> bar` gives `// foo` newline `// <caret>bar`; `// foo<caret>` gives `// foo` newline `<caret>`. |
| ED-TYPE-067 | MUST | **Block and documentation comments**: Enter inside a documentation comment (`/** ... */`) continues with the line prefix (` * `) aligned with the previous line's asterisk, with the caret after it; inside a plain block comment it continues with the indent of the previous line (and a ` * ` prefix only if that line has one). With `Close block comment` (`CLOSE_COMMENT_ON_ENTER`, default on), Enter after an **unterminated** `/*` or `/**` line inserts the closing line (` */`) below. With `Insert documentation comment stub` (`JAVADOC_STUB_ON_ENTER`, default on), Enter after `/**` before a declaration generates a stub with the declaration's parameter and return tags. | Java: in a doc comment line ` * foo<caret>` Enter gives ` * foo` newline ` * <caret>`; `/**<caret>` before a method gets the closing ` */`. |
| ED-TYPE-068 | SHOULD | Enter inside a string literal splits it according to the language (Java: closes the literal and joins with `+`, Java text blocks: plain newline). | Java: `"ab<caret>cd"` as an argument gives `"ab"` newline `+ "cd"` with continuation indent. |
| ED-TYPE-069 | MUST | `Start New Line` (Shift+Enter): removes the selection, moves the caret to the end of its line and presses Enter (smart indent applies). `Start New Line Before Current` (Ctrl+Alt+Enter): moves to the line start, presses Enter, then returns to the empty line created above and moves to its end; it indents that line. At the last line the empty line still gets created. | `a<caret>b` newline `cd`, Shift+Enter gives `ab` newline `<caret>` newline `cd`. |
| ED-TYPE-070 | MUST | `Split Line` (Ctrl+Enter): when only blanks precede the caret on the line it inserts a copy of that blank prefix plus a newline above (the caret stays with its text); otherwise it does Enter and returns the caret to where it was before the break (so the caret stays at the end of the first part). Not enabled in dialogs. | Plain: `ab<caret>cd` Ctrl+Enter gives `ab<caret>` newline `cd`. |
| ED-TYPE-071 | MUST | An Enter handler failure never leaves the document partly modified: the document change is one command, undoable at once (`Undo` restores the text before Enter). | Undo after Enter restores the line. |

## 6. Tab, Shift+Tab, indent

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-TYPE-080 | MUST | `Tab` with **no selection** (and no template/tab-out/lookup action taking it): inserts a tab character if `Use tab character` is on for the file (and, when `Smart tabs` is on, only if everything before the caret on the line is tabs), otherwise inserts spaces up to the next multiple of the **indent size** (not the tab size) counted from the caret column (or from the selection start column). It is disabled in one-line editors, viewers and dialog-embedded editors. | Indent 4, spaces: `a<caret>` Tab gives `a   <caret>`; column 4: Tab gives 4 spaces. |
| ED-TYPE-081 | MUST | `Tab` with a selection that contains a non-blank character (`Indent Selection`): indents every line touched by the selection by one indent unit (the indent size; spaces or tabs per file settings) and keeps the selection; a selection ending exactly at a line start does not include that last line; empty lines are indented too only when the language says lines can be indented there (lines inside multi-line strings and comments where the language says no are skipped); if the selection started at a line start and ends at a line end or line start, the selection is extended to cover the new leading indentation. A blank-only selection is replaced by a tab step (`Tab` as without selection). | Plain: `<selection>a`/`b</selection>` gives both lines with 4 more spaces, still selected. |
| ED-TYPE-082 | MUST | `Shift+Tab` (`Unindent Line or Selection`) unindents the caret line or all selected lines by one indent unit (removing at most the leading whitespace); enabled even without a selection; a line with less indent than one unit loses all of it. The caret moves with its text. | `    a<caret>` Shift+Tab gives `a<caret>`; two selected lines each lose 4 spaces. |
| ED-TYPE-083 | SHOULD | `Indent Line or Selection` (Tab in some keymaps, no default key here) indents the current line even without a selection. | Action `EditorIndentLineOrSelection` on `a<caret>` gives `    a<caret>` (the caret keeps its position in the text). |
| ED-TYPE-084 | MUST | Bulk indent of more than 50 lines runs as one bulk document operation (one undo step, one change notification). | 100-line selection, Tab, one Undo. |
| ED-TYPE-085 | MUST | Whether tab characters or spaces are produced, the indent size, tab size and `Smart tabs` come from the file's code style (`codeinsight/08` section 7 and file 08 of this set). Defaults: indent size 4, tab size 4, use tab character off (language style may differ, Java default is 4/4/off). | Java file: Tab inserts 4 spaces. |

## 7. Complete Current Statement (Smart Enter)

Action `EditorCompleteStatement`, key **Ctrl+Shift+Enter**, text `Complete Current Statement`.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-TYPE-090 | MUST | The action: closes an active lookup, ends an active live template, and if the caret is right after an unmatched `{` behaves as Enter. Otherwise it runs the language's smart-enter processors in order until one reports success; if there is none or none succeeds it does `Start New Line` (Shift+Enter behavior). In one-line editors and without a project it does a plain Enter. It is one undo command named after the action. | Plain text file: Ctrl+Shift+Enter acts like Shift+Enter. |
| ED-TYPE-091 | MUST | A language smart-enter processor takes the statement or declaration at the caret and applies a fixed-order list of **fixers**; after any fixer that changes the document the syntax tree is committed and the whole process restarts from the first error position, at most **20 attempts**; exceeding that restores the exact document text from before the action. When all fixers are done it moves the caret to the place where the user continues typing (inside a new block body, or the next line) via the enter processors. | A file where fixers loop: text restored after 20 attempts. |
| ED-TYPE-092 | MUST | Java fixers, in order: literal (closes an unterminated string), method call (adds `)`), if condition, for statement, ternary colon, while condition, catch declaration, switch expression, switch label colon, do-while condition, block brace, missing if branches, missing try body, missing switch body, missing lambda body, missing catch body, missing synchronized body, missing loop body, parameter list, missing comma, missing method body, missing class body, missing return expression, missing throw expression, parenthesized, semicolon, missing array initializer brace, missing array constructor bracket, enum field. Enter processors, in order: comment breaker, after-semicolon, leave code block, plain enter. | Cases of ED-TYPE-092: `if (a<caret>` gives `if (a) {` newline `<caret>` newline `}`; `foo(a<caret>` gives `foo(a);` with the caret on a new line after it. |
| ED-TYPE-093 | MUST | After a successful complete of a statement ending in a semicolon, the caret goes to the end of the next line start (new line, indented like the statement). If the caret is inside a code block that is complete, the action moves the caret out after the closing brace and inserts a new line. | `int x = 1<caret>` gives `int x = 1;` newline `<caret>`. |
| ED-TYPE-094 | SHOULD | Inside comments or doc comments the processors defer to the comment enter rules (ED-TYPE-066/067). | `// foo<caret>` Ctrl+Shift+Enter inserts a new comment line following ED-TYPE-066 or a plain new line. |

## 8. Paste

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-TYPE-100 | MUST | Paste (Ctrl+V, Shift+Insert) inserts the clipboard text at every caret: with one caret and no column mode in a file with syntax support, the **smart paste** below applies; with several carets or column mode, plain insertion: if the clipboard holds as many lines as there are carets, line *i* goes to caret *i* (carets ordered by position), else each caret gets the whole text. The selection is replaced. Without a project or syntax support the text is inserted verbatim. Content larger than the paste limit is refused with `Cannot Paste` / `Content is too large ({0}), limit is {1}`. Command name `Paste`; progress title `Pasting...`. | Cases. |
| ED-TYPE-101 | MUST | Line separators of pasted text are normalized to `\n`. | Paste CRLF text: document has `\n` only. |
| ED-TYPE-102 | MUST | **Reformat on paste** (`Reformat on paste:` `None`, `Indent block`, `Indent each line`, `Reformat block`; default **Indent each line**): after insertion the pasted range is (None) left as is; (Indent block) shifted so the pasted block's first line sits at the caret's column (the block anchor column is the selection start if a selection was replaced, else the caret column) while keeping the pasted lines' relative indents; (Indent each line) each pasted line is re-indented by the code style; (Reformat block) the formatter runs over the range. Without a formatter for the file, any mode other than None degrades to Indent block. Whitespace-only pastes and files too large for analysis are not reformatted. Pre-processors that changed the text (for example escaping when pasting into a string literal) force Reformat block. | Java: pasting `a();` and `b();` on two lines into a method body indents both lines to the body indent. |
| ED-TYPE-103 | MUST | Paste into a string literal escapes the text for the literal (newlines become `\n` plus continuation, quotes get backslashes) when the language provides a paste pre-processor (Java, Kotlin and most languages with string literals do; the user-visible option for it is an open question). | Java: pasting `a"b` into `"<caret>"` gives `"a\"b<caret>"`. |
| ED-TYPE-104 | MUST | Text copied from an **empty selection** (whole line copied by Copy with nothing selected) is pasted according to the setting (Settings, Editor, General: `Paste above the caret line`, `Paste at the caret position`, `Trim if pasted middle line`; advanced setting `When pasting a line copied with no selection`, default **Paste above the caret line**): above-the-line mode inserts the whole lines at the line start of the caret line and keeps the caret where it was relative to its text; at-caret mode inserts at the caret. | Copy a line with no selection, move to another line, Paste: the line appears above it. |
| ED-TYPE-105 | SHOULD | Paste is followed by `Insert imports on paste` (`Always`, `Ask`, `Never`; default **Always**) for code copied from another file, and by folding-state transfer (regions folded in the source stay folded), both via the copy-paste post-processor chain (`CI-OPT-060`). One undo step covers paste, indent and imports. | Copy Java with a reference to a class in another package: imports are added on paste. |
| ED-TYPE-106 | MUST | `Paste as Plain Text` (Ctrl+Alt+Shift+V) inserts raw text with no processing, reformat or escaping. `Paste from History` (Ctrl+Shift+V, Ctrl+Shift+Insert) shows the clipboard history chooser. `Paste from X Clipboard` (middle mouse) pastes the primary selection. | Raw paste keeps odd indentation. |
| ED-TYPE-107 | MUST | The caret ends after the pasted text and the viewport scrolls to it; the pasted range is remembered as the editor's last pasted region (used by `Undo` of reformat and by the paste highlight). | Caret at paste end. |

## 9. Settings and defaults

Settings, Editor, General, Smart Keys (labels from the message bundle) and the Enter group; the
values are the product defaults from the settings classes.

| Setting (label) | Default |
|---|---|
| Surround selection on typing quote or brace | on |
| Insert pair quote | on |
| Insert paired brackets (), [], {}, <> | on |
| Insert pair '}' (on Enter) | on |
| Jump outside closing bracket/quote with Tab when typing | on |
| Smart indent (on Enter) | on |
| Close block comment (on Enter) | on |
| Insert documentation comment stub (on Enter) | on |
| Home moves caret to first non-whitespace character | on |
| End on blank line moves caret to indent position | on |
| Use "CamelHumps" words | off |
| Reformat on paste | Indent each line |
| Insert imports on paste | Always |
| Unindent on Backspace | To proper indent position |
| Reformat block when typing the closing brace | on |
| Wrap when typing reaches right margin | off |
| Auto-popup completion on typing | on |
| Auto-popup parameter info | on; delay 1000 ms (range 0 to 5000) |
| Auto-popup documentation | off; delay 500 ms (range 0 to 5000) |

## 10. Open questions

* The exact Java set of tokens before which paired braces are not allowed (taken as the list in
  ED-TYPE-010; the shared helper may differ for Kotlin and others).
* The name and default of the setting that escapes pasted text into a string literal (ED-TYPE-103).
* Exact formatter spacing rules for the join in smart Backspace (ED-TYPE-046) in each language.
* Kotlin, JavaScript, Python and markup-specific typed handlers (`>` after a tag, `=` in an
  attribute): specified with each language.

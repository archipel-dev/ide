# Editor spec 09: code style inputs of the editor

Status: specification, language independent. Requirement prefix `ED-STYLE`.
Priorities: `MUST` / `SHOULD` / `LATER`.

The formatter engine, the optimize-imports framework, Reformat Code and the dialogs are specified in
`../codeinsight/08-optimize-imports-and-reformat.md` (`CI-OPT`); this file adds only what the
**editor** reads from the code style and from the file, and the rules that decide which indentation
setting applies to a given file at a given moment. It does not repeat the table of typed and paste
hooks in `CI-OPT-060`; the detailed contracts of those hooks are in `02-typing-and-typed-handlers.md`.

Executable acceptance cases are in `cases/09-code-style-interplay.cases.md` (format `cases/README.md`).

ic: `platform/code-style-api/src/com/intellij/psi/codeStyle/{CodeStyleSettings,CommonCodeStyleSettings,
CodeStyleDefaults,CodeStyleConstraints,FileIndentOptionsProvider}.java`,
`platform/lang-impl/src/com/intellij/psi/codeStyle/{DetectableIndentOptionsProvider.java,
autodetect/IndentOptionsDetectorImpl.java}`, `platform/platform-impl/src/com/intellij/openapi/editor/impl/
EditorSettingsState.kt`, `platform/ide-core/resources/messages/ApplicationBundle.properties`.

## 1. Values the editor reads

| Value | Used by | Default |
|---|---|---|
| Indent size | Tab, Shift+Tab, Enter, Backspace unindent, indent selection, paste indent | 4 (allowed 0 to 32) |
| Tab size | rendering of tab characters, Home/End and column math | 4 (allowed 1 to 16) |
| Use tab character | Tab key output, indent conversion | off |
| Smart tabs | Tab key when tabs are used for indent and spaces for alignment | off |
| Continuation indent | Enter inside an unfinished expression, wrapped arguments | 8 |
| Label indent, absolute label indent | Enter after a label | 0, off |
| Keep indents on empty lines | whether Enter and unindent leave whitespace on blank lines | off |
| Right margin | the margin guide, `Wrap when typing reaches right margin`, fold one-liner fit | 120 (hard wrap), maximum 1000 which also hides the guide |
| Wrap when typing reaches right margin | auto hard wrap while typing | off |
| Soft (visual) margins | extra guides | none |
| Line separator for new files | the separator of files created by the IDE | system dependent (the document itself always uses `\n`) |
| Line comment at first column, add space after the comment prefix, block comment add space | `ED-LINE-041`, `ED-LINE-044` | on, off, off |
| Formatter on/off tags enabled | the editor does not format ranges between formatter off/on comments | on (a language option) |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-STYLE-001 | MUST | The editor reads these values **per file and per caret language**: the settings of the file's language, or, in a multi-language file or an injected fragment, of the language at the caret for indentation commands (Backspace unindent uses the language at the caret, Tab uses the file's options). Changing a code style setting takes effect on the next command in open editors without reopening them. | Change indent size to 2: the next Tab inserts spaces to a multiple of 2. |
| ED-STYLE-002 | MUST | Precedence for a file's indent options, highest first: (1) an indent override attached to the file by a plugin or by the editor (for example the status-bar `n spaces` widget choice for the file); (2) **EditorConfig** properties (`indent_style`, `indent_size`, `tab_width`, `max_line_length`, `end_of_line`, `trim_trailing_whitespace`, `insert_final_newline`) found for the file when EditorConfig support is on (default **on**); (3) **detected indents** (section 2); (4) the project code style scheme for the file's language; (5) the global defaults of section 1. A value an EditorConfig file does not mention comes from the next source. | A `.editorconfig` with `indent_size = 2` and an unconfigured tab size: indent 2, tab size from the scheme. |
| ED-STYLE-003 | MUST | Defaults are those of section 1; each language may set its own default through its code style provider (for example 2 spaces for some web languages), and the code style scheme can be project level (default) or global; the code style has the schemes `Default` and `Project`. | A new JSON file indents by the JSON language default. |
| ED-STYLE-004 | SHOULD | Right-margin related behaviors: the margin guide (`ED-APPEAR-042`), `Wrap when typing reaches right margin` (auto hard wrap on typing a character beyond the margin: the line breaks at the last allowed place before the margin, with the formatter indent, one undo step with the typed character), and Join Lines spacing use the language's right margin when set and otherwise the global 120. | Typing past column 120 with wrap on breaks the line. |

## 2. Indent detection

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-STYLE-010 | MUST | With `Detect and use existing file indents for editing` (default **on**; stored `AUTODETECT_INDENTS`) the editor derives the indent size, tab size and tab character use of a **writable** file from its content: only for languages that have a formatter, not for scratch or light (in-memory) files, not for files over 1 MB, and only when the file has at least 3 lines. Detection runs in the background after the document is committed and the result is cached per document and discarded when the document changes substantially; until it finishes the scheme's options apply. The detected options are used for editing commands (Tab, Enter, Backspace, indentation actions) but **not for a full-file reformat** (reformat uses the scheme options). The status bar shows the detected indent and lets the user override it. | A 2-space file in a 4-space scheme: Enter after `{` indents by 2. |
| ED-STYLE-011 | MUST | The detector builds the line indent information from the formatter's block model (so continuation lines and aligned parameters are not mistaken for block indents), counts how often each indent step occurs, and picks the indent size with the largest share; a file with mostly tab-indented lines selects use-tab; a file with inconsistent indents keeps the scheme options. | A file indented by 3 spaces gets indent 3. |
| ED-STYLE-012 | SHOULD | `Detect and use existing file indents` can be turned off per file from the status bar widget; the discarded options are remembered for the file for the session. | Widget: `Disable indent detection`. |

## 3. Interplay with typing and paste

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-STYLE-020 | MUST | An editing command that changes indentation (Enter, Tab, Shift+Tab, Backspace unindent, paste with `Indent block` or `Indent each line`, Move Line with statement mover, Auto-Indent Lines) asks the file's effective options (ED-STYLE-002); an indentation command never mixes tabs and spaces beyond what `Use tab character` and `Smart tabs` prescribe. With tabs on and smart tabs off, leading indentation uses tab characters for full tab stops and spaces for the remainder only when the indent size is not a multiple of the tab size. | Tabs on, indent 4, tab 4: Tab gives one tab character. |
| ED-STYLE-021 | MUST | Reformat-on-typing hooks (closing brace, paste, Enter) run the formatter only on the minimal range they touch and never reformat outside the line or block they were triggered for (`CI-OPT-060`); formatter-off tags and `@formatter:off` regions are respected: no reformatting and no re-indentation inside them. | A typed `}` inside a formatter-off region is not re-indented. |
| ED-STYLE-022 | MUST | The caret line is kept when the format touches it: after any reformat triggered by typing, the caret stays on the same token and the viewport does not jump. | Caret does not move relative to its token after `}`. |

## 4. Open questions

* The exact list of EditorConfig properties applied to non-indent behaviors (line endings, final
  newline) is specified with saving (`platform/09-saving-and-autosave.md` when it lands).
* Which languages ship non-4 defaults.
* The relative priority of EditorConfig and indent detection in `ED-STYLE-002` is taken from product
  behavior; the provider ordering was not read in the sources.

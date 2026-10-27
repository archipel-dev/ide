# 01. Find and Replace in the editor (the search bar)

Status: specification, language independent. Requirement prefix `SRCH-FIL`.
Source basis: IntelliJ Community tag `idea/262.10968.63`. Paths are cited as `ic: ...`.
Priorities: `MUST` / `SHOULD` / `LATER`. Executable cases: `cases/01-find-replace-in-file.cases.md`.

Scope: `Ctrl+F` / `Ctrl+R` in an editor, the bar above the text, F3 / Shift+F3 and the word-at-caret commands, replace and
replace all, preserve case, regex replacement syntax, multi-caret selection of occurrences, search history. Find in Files is
`02-find-in-files.md`; the text matcher for names is `07-matching-and-ranking.md` (not used here: text search is plain
substring / regex search, see section 3).

ic: `platform/lang-impl/src/com/intellij/find/EditorSearchSession.java`, `.../SearchReplaceComponent.java`,
`.../FindUtil.java`, `.../impl/FindManagerBase.java`, `.../impl/FindManagerImpl.java`,
`.../impl/livePreview/{LivePreviewController,SearchResults,LivePreview}.java`, `.../impl/PreserveCaseUtil.java`,
`.../impl/RegExReplacementBuilder.java`, `.../editorHeaderActions/*`, `platform/indexing-api/src/com/intellij/find/FindModel.kt`,
`platform/analysis-impl/resources/messages/FindBundle.properties`, `platform/ide-core/resources/messages/ApplicationBundle.properties`
(keys `editorsearch.*`), `platform/util/resources/misc/registry.properties` (keys `ide.find.*`).

## 1. Actions and default keys

Keymap: the default keymap (`$default`); `Ctrl` is `Cmd` on macOS where the platform keymap differs.

| Action id | Text | Keys | Notes |
|---|---|---|---|
| `Find` | Find… | `Ctrl+F`, `Alt+F3` | opens or focuses the bar in find mode |
| `Replace` | Replace… | `Ctrl+R` | opens or focuses the bar in replace mode |
| `FindNext` | Find Next / Move to Next Occurrence | `F3`, `Ctrl+L` | |
| `FindPrevious` | Find Previous / Move to Previous Occurrence | `Shift+F3`, `Ctrl+Shift+L` | |
| `FindWordAtCaret` | Next Occurrence of the Word at Caret | `Ctrl+F3` | |
| `FindPrevWordAtCaret` | Previous Occurrence of the Word at Caret | `Ctrl+Shift+F3` | |
| `ToggleFindInSelection` | Search in Selection Only | `Ctrl+Alt+E` | |
| `ShowSearchHistory` | Show Search History | `Alt+Down` | in the bar fields |
| `SelectAllOccurrences` | Select All Occurrences | `Ctrl+Alt+Shift+J` | `Alt+Enter` inside the bar |
| `SelectNextOccurrence` / `UnselectPreviousOccurrence` | Add Selection for Next Occurrence / Unselect | `Alt+J` / `Alt+Shift+J` | editor actions, also bar buttons |
| `ToggleScrollToResultsDuringTypingAction` | Scroll to Results During Typing | none | setting, default on |
| `EditorSearchSession.ToggleMatchCase` / `ToggleWholeWordsOnlyAction` / `ToggleRegex` | Match Case / Words / Regex | `Alt+C` / `Alt+W` / `Alt+X` (mnemonics) | bar toggles |
| `SearchContext.Anywhere` / `InComments` / `InStringLiterals` / `ExceptComments` / `ExceptStringLiterals` / `ExceptCommentsAndStringLiterals` | the *Filter Search Results* menu | none | section 6 |
| `FindUsagesInFile` | Find Usages in File | `Ctrl+F7` | specified in `03-find-usages.md` |
| `FindSelectionInPath` / `FindInPath` / `ReplaceInPath` | Find / Replace in Files | `Ctrl+Shift+F` / `Ctrl+Shift+R` | `02-find-in-files.md` |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-FIL-001 | MUST | Every action in the table exists with the stated default shortcut and appears in *Find / Replace* menus and in Search Everywhere Actions. A toggle shows its checked state. | keymap test; SRCH-FIL-002 cases |
| SRCH-FIL-002 | MUST | `Ctrl+F` in an editor opens the **search bar** as the editor's header (above the text, inside the editor tab). If the bar is open: focus moves to the search field and its text is selected; if it was in replace mode it switches to find mode (the replace field disappears). `Ctrl+R` opens the bar in replace mode, or switches an open bar to replace mode, and selects the search text. Tool-window consoles and read-only viewers allow find but not replace (replace controls hidden). | cases FIL-002/1..3 |
| SRCH-FIL-003 | MUST | `Esc` in the bar closes it and returns focus to the editor; the editor keeps the selection and caret the search left (the last match selected, caret at its end). `Esc` also closes when the search field is empty. `Ctrl+Enter` (`Cmd+Enter`): if the field is empty, close; otherwise focus the editor, keep the bar, add the text to the history. | cases FIL-003/1..2 |

## 2. Bar layout, texts, states

The bar is one component with: the **search field** (multi-line capable, grows to at most 3 rows, registry `ide.find.max.rows`,
vertical scroll beyond); inside the field at the right the three toggles *Match Case*, *Words*, *Regex* (icon buttons with
tooltips `Match Case` / `Words` / `Regex`, mnemonic hints `Alt+C/W/X`), and the history dropdown; to the right of the field the
result counter and the actions *Previous Occurrence*, *Next Occurrence*, *Filter Search Results*, *Search in Selection*,
and a *More* menu (`Open in Window, Multiple Cursors`: *Find All* / `Open in Find Window`, `Add Selection for Next Occurrence`,
`Unselect Last Occurrence`, `Select All Occurrences`, `Scroll to Results During Typing`); a close button with tooltip
`Close search bar (Escape)`. In replace mode a second row shows the **replace field** (placeholder `Replace`) with toggle *Pr&eserve case*
(mnemonic `Alt+E`) inside it and the buttons `Replace` (`Alt+P`), `Replace All` (`Alt+A`), `Exclude`/`Include` (`Alt+L`). A button
at the left edge toggles replace mode (tooltip `Show Replace Field` / `Hide Replace Field`). Placeholder texts:

| Situation | Search field placeholder |
|---|---|
| no option on | `Search` |
| one option on | the option name lower-cased: `match case`, `words`, `regex` |
| two options on | `<a> and <b>` in the order case, words, regex |
| search in selection, selection single-line | `Search in selection only` |
| search in selection, selection multi-line | `Search in selection. Ctrl+Alt+E to search for selected text` (the shortcut of the toggle; of `Replace` when in replace mode) |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-FIL-010 | MUST | **Counter.** While the search runs on a non-empty field the counter shows: `i/N` when matches exist (1-based index of the current match among all `N` matches, in document order; a current match always exists when `N > 0`); `0 results` when there are none (the forms `N results` / `1 result` exist for the short transient state in which matches are known but no current match is chosen yet); `N+` when more than 10000 matches exist (limit `10000`; a link `Click to highlight` appears and raises the limit for this session); `No selection` when search in selection is on, no text is selected and nothing matched; `Bad pattern` (regex, invalid expression); `Vague pattern` (regex consisting only of `|` characters). An empty field shows `0 results` and no highlighting. | cases FIL-010/1..5 |
| SRCH-FIL-011 | MUST | **Not-found state.** When a non-empty valid search finished with zero matches the field background becomes the *not found* (error) color; with an invalid regex too. It returns to normal as soon as matches exist or the field is emptied. A search still running that has found nothing yet does not show the error state. | cases FIL-011/1 |
| SRCH-FIL-012 | MUST | **Live search.** Every change of the field (typing, paste, option toggle, scope change) re-runs the search after at most 30 ms of quiet (live-preview delay), highlights *all* matches with the *search result* highlight and the current match with the *current occurrence* frame, and updates the counter. The first search after opening runs without delay. Highlights stay as long as the bar is open and disappear when it closes. | cases FIL-012/1 |
| SRCH-FIL-013 | MUST | **Scroll to results during typing** (default on). While typing, the editor selection and caret follow the current match (it is selected and scrolled into view). With the setting off, typing only highlights; the caret and selection stay until the user navigates with Enter or F3. | cases FIL-013/1..2 |
| SRCH-FIL-014 | MUST | **Initial text.** Opening with a **single-line selection** puts the selection text into the field (selected) and searches. With a **multi-line selection** the bar opens in *search in selection* mode with an empty field. With no selection, the field shows the last search text of the previous session (selected, so typing replaces it) and the toggles keep their last values; setting `ide.find.select.word.at.caret` (default off) would use the word at the caret instead. Selected text longer than 10000 characters is not copied into the field. In regex mode a copied selection is regex-escaped (registry `ide.find.escape.selected.text.for.regex`, default on). | cases FIL-014/1..3 |
| SRCH-FIL-015 | SHOULD | The option toggles `Match Case`, `Words`, `Regex` and the scope filter are remembered per application between sessions (separate settings for the editor bar and for Find in Files); *Preserve case* is remembered too. When the field is empty and a previous find model exists, the field's left action `Restore previous settings` (key `Enter`) restores the previous query and options. | cases FIL-015/1 |

## 3. What a match is

The search model has: the text to find; flags *case sensitive* (default **off**), *whole words* (default off), *regular expression*
(default off), *search context* (default *anywhere*), *in selection* (default off), *preserve case* (default off).

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-FIL-020 | MUST | **Plain search** finds every occurrence of the text, case-insensitive unless *Match Case* is on. Occurrences found by scanning left to right do not overlap: after a match the search for the next continues at the match end (`aa` in `aaaa` gives 2 matches). Case-insensitive comparison uses Unicode simple case folding per character. | cases FIL-020/1..3 |
| SRCH-FIL-021 | MUST | **Whole words.** A candidate occurrence `[s,e)` is accepted when its start is a *word start* and its end a *word end*. Word start: `s` is 0, or if the first char is an identifier part (letter, digit, `_`, `$` and other Java-identifier-part chars) the previous char is **not** an identifier part, ignoring a previous identifier char that is itself preceded by a backslash (so `foo` is a whole word in `\nfoo`); if the first char is not an identifier part, the previous char must differ from the first char. Word end: `e` is the text length, or if the last char is an identifier part the next char is not an identifier part; otherwise the next char must differ from the last char. (`==` is a whole word in `a == b` but not inside `===`.) Rejected candidates are skipped and the scan continues one char after their start. | cases FIL-021/1..3 |
| SRCH-FIL-022 | MUST | **Regex** uses Java `java.util.regex` syntax and semantics (named groups `(?<n>…)`, lookaround, lazy and possessive quantifiers, backreferences, `\b`, `\p{…}`, inline flags). Flags: `MULTILINE` always (so `^` and `$` match at every line boundary); `CASE_INSENSITIVE | UNICODE_CASE` unless *Match Case*. A pattern containing `\n` may match across lines. A match end beyond the text is discarded. A regex that overflows the matcher stack is treated as no match and reported once per pattern as a notification `Regular expression failed to match` (`'<p>' produced stack overflow when matching content of the file '<file>'`). *Whole words* is applied on top of the regex matches with the rule of 021. | cases FIL-022/1..4 |
| SRCH-FIL-023 | MUST | **Empty matches.** A regex match of zero length is highlighted as nothing, is not counted, and never stops the scan (the scan advances one char). A pattern that only matches the empty string shows `0 results`. | case FIL-023/1 |
| SRCH-FIL-024 | MUST | **Multi-line text.** A search string containing a line break matches across lines; the field becomes multi-line (2 rows). Line breaks in the text are `\n` internally whatever the file's line separator. | case FIL-024/1 |
| SRCH-FIL-025 | MUST | **Search in selection** (toggle `Search in Selection`, `Ctrl+Alt+E`, or automatic for a multi-line selection): only matches fully inside the current selection (all selection blocks for a column selection) are found, highlighted and counted; matches outside are not highlighted. The selection stays. Changing the selection updates the results. Turning the toggle **on** restores the caret and selection the editor had when the bar opened (so a selection moved by the live search is brought back), or clears the field when its text equals the text that was originally selected. Turning it **off** with an empty field copies the current selection text into the field and searches the whole document. | cases FIL-025/1..2 |

## 4. Navigation: Enter, F3, wrap-around

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-FIL-030 | MUST | **Next / previous in the bar.** `Enter` and `F3` (and `Down` in a single-line field) go to the next match, `Shift+Enter`, `Shift+F3` (and `Up`) to the previous. The next match is the first with start >= the caret (the caret sits at the end of the current match); the previous is the last match whose end <= the start of the current one. The new current match is selected, scrolled into view, the counter updates. Disabled (no effect) while the search is running or with no matches. The fields keep focus. | cases FIL-030/1..3 |
| SRCH-FIL-031 | MUST | **No silent wrap.** At the last match, `Enter`/`F3` does not move; it shows a hint under the caret: `"<text>" not found, press F3 to search from the top` (the key text is the first keyboard shortcut of *Find Next*; without a shortcut: `"<text>" not found, perform "Find Next" again to search from the top`). The *next* press wraps to the first match. Symmetric for backwards at the first match: `"<text>" not found, press Shift+F3 to search from the bottom` (*Find Previous*) then wraps to the last. The hint hides on any key, text change or scroll. Typing in the field or moving the caret resets the wrapped state. | cases FIL-031/1..2 |
| SRCH-FIL-032 | MUST | **First match after typing.** After the first search of a query the current match is the first match at or after the caret (the match containing the caret counts, inclusive of its end); if there is none after the caret, the first match of the document silently, without hint. | cases FIL-032/1..2 |
| SRCH-FIL-033 | MUST | **F3 / Shift+F3 with the bar closed** use the last search model: the next occurrence after the caret (or before for Shift+F3); with a selection that equals a match search continues from the selection end. If no find was ever done in the project session, `F3` opens the bar instead. Not found: the hint of 031 appears under the caret and the next `F3` restarts from the top (from the bottom for backwards). The found match is selected and the caret placed at its end; highlighting is not shown (no bar). | cases FIL-033/1..2 |
| SRCH-FIL-034 | MUST | **Word at caret.** `Ctrl+F3` searches forward for the identifier at the caret (maximal run of identifier-part chars containing the caret or ending at it; with a selection, the selection text); `Ctrl+Shift+F3` backward. The search is **case-sensitive, not whole-word**, starts at the caret, selects the found occurrence (caret at its end). With the bar open the word also becomes the field text. No identifier at the caret: nothing happens. Not found: hint of 031; `F3` then wraps. The word is added to the search history and becomes the model of later `F3`. | cases FIL-034/1..3 |
| SRCH-FIL-035 | MUST | Every selection movement made by find navigation is added to the navigation history (Back / Forward, see `05-recent-and-structure-popups.md`). | covered by 05 cases |

## 5. Replace

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-FIL-040 | MUST | **Replace** (button, `Enter` in the replace field, `Alt+P`): replaces the current match by the replacement text (after regex/preserve-case processing) and moves to the next match after the replacement. Disabled when there is no current match. The change is one undoable command named `Replace`. The document must be writable (read-only files are requested for writing as the platform does). | cases FIL-040/1..2 |
| SRCH-FIL-041 | MUST | **Replace All** (`Alt+A`): replaces every match in the document (or in the selection when search in selection is on), **left to right on the original text, without rescanning the replacement** (`a`→`aa` over `aaa` gives `aaaaaa`). Excluded matches (040 Exclude) are left alone. Empty matches advance one char. It is one undo step and one *Local History* action labelled `Replace all '<find>' in file with '<replacement>'`. Afterwards the caret stays at its adjusted position, the selection is removed (global mode), and the status bar shows `N occurrences replaced` (`1 occurrence replaced`; nothing for 0). The bar stays open. | cases FIL-041/1..4 |
| SRCH-FIL-042 | MUST | **Exclude / Include** toggles the exclusion of the current match from Replace All, shown as a struck/dimmed highlight; the button text flips between `Exclude` and `Include`; it then moves to the next match. Exclusions are dropped when the text of the match changes. | case FIL-042/1 |
| SRCH-FIL-043 | MUST | **Replacement syntax with regex on**: the replacement is a template where `$n` and `${name}` insert a group (`$0` is the whole match; two-digit `$12` is taken only if that group exists, otherwise `$1` followed by `2`); a group that did not take part inserts nothing; `\n \r \t \f \b` insert the control characters, `\xNNNN` (exactly four hex digits) the code unit; `\\` and any other `\c` insert `c`; `\U … \E` upper-case, `\L … \E` lower-case the following text (until `\E` or the end), `\u` and `\l` change only the next inserted character. The first `\U`/`\L` ends an open one of the other kind. Without regex the replacement is literal (no escapes, `$` and `\` are literal). | cases FIL-043/1..6 |
| SRCH-FIL-044 | MUST | **Malformed template** (regex on): at Replace / Replace All an error dialog titled `Replace Error` shows `Malformed replacement string: <reason>` and nothing changes. Reasons: `character to be escaped is missing` (template ends with `\`), `Illegal group reference: group index is missing` (ends with `$`), `Illegal group reference` (`$` followed by a non-digit, not `{`), `named capturing group has 0 length name`, `named capturing group is missing trailing '}'`, `capturing group name {1x} starts with digit character`, `No group 3` (group number larger than the group count of the pattern, also reported with an unknown name as `No group with name <x>`). | cases FIL-044/1..3 |
| SRCH-FIL-045 | MUST | **Preserve case.** With *Preserve case* on, the replacement text takes the capitalisation of each found text (word-based algorithm, registry `ide.find.word.based.preserve.case` default on): the found text is split into words (runs of letters/digits); for each word of the replacement, in order, the case pattern of the matching found word (the last found word is reused when the replacement has more words) is applied: all-upper found word gives an all-upper replacement word; a found word that is lower-case gives a lower-case word; a capitalised found word (first upper, rest lower or mixed) gives a capitalised word with the rest of the replacement word lower-cased when it was all upper, otherwise unchanged; a found word without letters leaves the replacement word unchanged. Non-letters in the replacement stay. With regex on, preserve case is applied after template expansion. Examples in the cases. | cases FIL-045/1..6 |
| SRCH-FIL-046 | SHOULD | Replace and Replace All treat a replacement that equals the match as a no-op that still counts; replacing never moves the caret outside the document; guarded / read-only fragments cancel the replacement with the editor's read-only-fragment message. | LATER cases |

## 6. Search context (filter)

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-FIL-050 | SHOULD | The *Filter Search Results* menu offers: `Anywhere` (default), `In Comments`, `In String Literals`, `Except Comments`, `Except String Literals`, `Except Comments and String Literals`; exactly one is active; the filter button is highlighted when the choice is not `Anywhere`. Matches are limited to the chosen category using the file's lexer (comment tokens, string literal tokens). Files without that information (plain text) match nothing for the `In …` choices and everything for the `Except …` choices. | cases FIL-050/1..2 |

## 7. Multi-caret from the bar

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-FIL-060 | MUST | **Select All Occurrences** (`Ctrl+Alt+Shift+J`, bar `Alt+Enter`, *More* menu) puts one caret with a selection on every match (all matches, or those in the selection when searching in selection), then closes the bar. The last match keeps the primary caret. | case FIL-060/1 |
| SRCH-FIL-061 | SHOULD | **Add / remove occurrence** from the bar (`Alt+J`, `Alt+Shift+J` and the *More* menu entries) adds the next match to the multi-selection or removes the last added one (bar stays open). The editor-level commands without a bar are specified with multi-caret editing. | case FIL-061/1 |
| SRCH-FIL-062 | SHOULD | *Find All* / `Open in Find Window` shows all matches of the file in the Find tool window (usage view, `03-find-usages.md` section 4) titled `Occurrences of '<text>'` plus the context suffix (` in String Literals`, ` in Comments`, ` Everywhere Except String Literals`, ` Everywhere Except Comments`, ` Everywhere Except Comments and String Literals`) and scope `File '<name>'`. | LATER cases |

## 8. History

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-FIL-070 | MUST | A search string is added to the *Search History* when the user navigates (Enter, F3, Next button), replaces, or presses `Ctrl+Enter`; a replacement string is added to the *Replace History* at replace. Entries are unique (re-adding moves to the newest), at most 300 per list, shared with Find in Files, persisted. `Alt+Down` in a field opens the history popup (newest first). Choosing an entry fills the field and searches. | cases FIL-070/1..2 |
| SRCH-FIL-071 | SHOULD | The field offers completion on `Ctrl+Space` with the words of the file matching the typed prefix as a camel-hump pattern (matcher of `07-matching-and-ranking.md`, IGNORE_CASE, hard separators none), sorted ascending. | case FIL-071/1 |

## 9. Behaviours around the bar

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-FIL-080 | MUST | The bar belongs to one editor; opening it in another editor creates another bar. Closing the editor disposes it. Highlights and markers are range markers that follow edits typed into the document while the bar is open; the results refresh after each edit. | case FIL-080/1 |
| SRCH-FIL-081 | SHOULD | While the bar is open, `Tab` / `Shift+Tab` move focus between the search field, replace field and buttons; `Space` toggles the focused option; the embedded buttons hint reads `Use Tab to focus on an option, and Space to toggle it`. | UI test |
| SRCH-FIL-082 | SHOULD | The find popup of the old *dialog* mode (`Find Text` / `Replace Text` dialogs with direction and origin options) is not required; the bar replaces it. | n/a |

## 10. Coverage of the inventory (`find-replace.md`)

Every action id of `docs/specs/inventory/find-replace.md` maps as follows. `EditorSearchSession.NextOccurrenceAction`,
`ToggleMatchCase`, `ToggleRegex`, `ToggleWholeWordsOnlyAction`: SRCH-FIL-002/020..022; `Find`: 002; `Replace`: 002/040; `FindNext`,
`FindPrevious`: 030/033; `FindWordAtCaret`, `FindPrevWordAtCaret`: 034; `FindInPath`, `ReplaceInPath`, `FindSelectionInPath`:
`02-find-in-files.md`; `FindUsages`, `FindUsagesInFile`: `03-find-usages.md`; `SearchContext.*` (6): 050; `ShowSearchHistory`: 070;
`ToggleFindInSelection`: 025; `ToggleScrollToResultsDuringTypingAction`: 013; `TextSearchAction`: `04-search-everywhere.md`
(SRCH-SE-060); `StructuralSearchPlugin.*`: `06-structural-search.md`; `ReplaceTempWithQuery`: refactoring area (not a search action).

## 11. Open questions

1. Regex flavor. The reference is `java.util.regex`. A Rust engine needs a compatibility layer for lookaround, backreferences
   and possessive quantifiers (the `regex` crate lacks the first two); the product direction says "own engine", so decide whether to
   ship a backtracking engine with Java semantics (recommended) or document differences.
2. Exact caret placement after closing the bar when the last navigation happened with the bar focused. The reference keeps the
   last match selected with the caret at its end; confirm with a recording.
3. Whether Replace All should get a keyboard shortcut in the bar; the reference binds only the mnemonic `Alt+A` there (the Find in
   Files popup uses `Alt+Shift+Enter`).

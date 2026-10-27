# 06. Code completion framework

Status: specification, language independent. Requirement prefix `CI-CMP`.
Priorities: `MUST` / `SHOULD` / `LATER`.

Language contributors (what Java/Kotlin/Spring offer in which context) belong to the language
specs. This file specifies the engine that runs them: how a completion session starts, how
contributors feed it, how items are matched, ordered, shown and inserted, and what keyboard
behaviour users rely on. Section 14 lists the UX acceptance scenarios.

ic: `platform/analysis-api/src/com/intellij/codeInsight/completion/` (`CompletionContributor`,
`CompletionParameters`, `CompletionType`, `CompletionResultSet`, `CompletionInitializationContext`,
`CompletionSorter`, `CompletionWeigher`, `PrioritizedLookupElement`, `InsertHandler`,
`InsertionContext`, `AutoCompletionPolicy`), `platform/analysis-api/src/com/intellij/codeInsight/lookup/`
(`LookupElement`, `LookupElementPresentation`, `LookupElementWeigher`, `LookupFocusDegree`,
`Lookup`), `platform/analysis-impl/src/com/intellij/codeInsight/completion/`
(`BaseCompletionService`, `BaseCompletionLookupArranger`, `impl/CamelHumpMatcher`,
`impl/CompletionSorterImpl`, `impl/RealPrefixMatchingWeigher`, `impl/PreferStartMatching`,
`impl/LiftShorterItemsClassifier`, `StatisticsWeigher`, `PriorityWeigher`, `GroupingWeigher`),
`platform/lang-impl/src/com/intellij/codeInsight/completion/` (`CodeCompletionHandlerBase`,
`CompletionProgressIndicator`, `WordCompletionContributor`, `DefaultCharFilter`),
`platform/lang-impl/src/com/intellij/codeInsight/{AutoPopupControllerImpl,editorActions/CompletionAutoPopupHandler}.java`,
`platform/lang-impl/src/com/intellij/application/options/CodeCompletionConfigurable.kt`,
`platform/util/text-matching/.../MinusculeMatcherImpl.kt`,
`platform/util/resources/misc/registry.properties`; Java examples:
`java/java-impl/.../completion/JavaCompletionContributor.java`.

## 1. Vocabulary

| Term | Meaning |
|---|---|
| **Completion session** | One run from "user asked for completion" to "lookup closed". It owns the offsets, the prefix, the result items and the lookup popup. |
| **Contributor** | A language- or feature-specific producer of items (keywords, locals, members, file paths, templates, LSP). |
| **Lookup element** | One candidate item: lookup strings used for matching, a presentation, an insert behaviour, an optional target (the symbol it stands for). |
| **Lookup** | The popup list of lookup elements plus the editor's "prefix" state (the part of the identifier typed so far). |
| **Prefix** | The text from the start of the identifier to the caret. Additional characters typed while the lookup is open extend it (*additional prefix*). |
| **Invocation count** | 0 for auto-popup; 1 for explicit invocation; 2 and more when the user invokes again while a lookup of the same type and position is showing. |
| **Completion type** | `BASIC` (all visible names), `SMART` (type-matching: only items that fit the expected type), `CLASS_NAME` (classes only, can import). |

## 2. Entry points and settings

### 2.1 Actions and shortcuts (default keymap)

| Action | Shortcut | Meaning |
|---|---|---|
| Basic Completion | `Ctrl+Space` (macOS: `Ctrl+Space`) | start BASIC with invocation count 1; pressing it again with the lookup open increments the count |
| Smart (type-matching) Completion | `Ctrl+Shift+Space` | start SMART; again = higher count |
| Class Name Completion | `Ctrl+Alt+Space` | classes by name, inserts the import |
| Cyclic Expand Word | `Alt+/` and `Alt+Shift+/` | text-only expansion, no popup (LATER) |
| Complete Current Statement | `Ctrl+Shift+Enter` | after the item is inserted, fix up syntax; also usable as a selection key in the lookup |
| Choose lookup item | `Enter`; replace: `Tab`; with dot: `Ctrl+.` | see 8 |
| Quick documentation in lookup | `Ctrl+Q` (macOS `F1`/`Ctrl+J`) | docs of the selected item |
| Quick definition in lookup | `Ctrl+Shift+I` | |

### 2.2 Settings (Settings | Editor | General | Code Completion; labels exact)

Section text: `Displays suggestions in a popup while typing, based on static analysis of the code`.

| Label | Meaning | Default |
|---|---|---|
| `Match case:` + `First letter only` / `All letters` | case sensitivity of prefix matching (checkbox off = none) | on, `First letter only` |
| `Automatically insert single suggestions for:` `Basic Completion` / `Type-Matching Completion` | when an *explicit* invocation yields exactly one item, insert it without showing the popup | both on |
| `Sort suggestions alphabetically` | disable relevance sorting and sort by item text | off |
| `Show suggestions as you type` | auto-popup | on |
| `Insert selected suggestion by pressing space, dot, or other context-dependent keys` (sub-option of auto-popup) | selection by "context keys" in auto-popup lookups | off |
| `Show the documentation popup in` `<N>` `ms` | auto-show docs next to the lookup | off; delay 500 ms (range 0 to 5000) |
| `Insert parentheses automatically when applicable` | after a method name is inserted | on |
| `Show parameter name hints on completion` | inline hints after inserting a call | off |

Registry-level switches (not shown in the page): middle matching on (suggest items that contain
the typed text anywhere), show non-imported classes that start with a lowercase letter off, show
live templates on top off, variant limit 1000, lookup list height 11 rows, max width 500 px,
preselect depends on context on.

`CI-CMP-001` MUST: the actions of 2.1 with these shortcuts. Acceptance: keymap data lists them.

`CI-CMP-002` MUST: the settings of 2.2 with these labels and defaults, persisted, applied to
the next session (and `Match case` also triggers a daemon restart). Acceptance: toggling
`Show suggestions as you type` stops/starts auto-popup immediately.

## 3. Session pipeline

### 3.1 Initialisation (UI thread, before any contributor)

1. Commit pending document changes to the syntax tree.
2. Create the *initialisation context*: editor, caret, file, completion type, invocation count,
   and an **offset map** with three offsets:
   - `start` = caret offset, or the selection start when there is a selection;
   - `selection end` = caret offset, or the selection end;
   - `identifier end` = the end of the run of identifier characters that follows `selection end`
     (the *default replacement end*).
   The offset map follows document edits.
3. Let every contributor of the language at the caret adjust it: change the *dummy identifier*
   or the offsets (`before completion` hook). Typical use: extend `identifier end` over a
   hyphenated name in CSS, or change the dummy text inside a string.
4. In parallel with the main work, contributors may refine the replacement offset
   (`during completion` hook), e.g. to the end of the reference at the caret (so that `Tab`
   replaces exactly the rest of the reference, not the longest identifier).
5. When the language has a reference at the caret, the *replacement offset* defaults to the end
   of that reference's range.

### 3.2 The completion copy and the dummy identifier

Completion never analyses the live, usually broken, tree. It creates a **copy of the file** in
which a fixed *dummy identifier* (a marker that cannot occur in real code and begins with a
letter, so the copy parses as if an identifier were being typed) is inserted at `start`
(replacing the selection) and runs the contributors against the copy:

- `position` = the leaf element in the copy at the caret (the dummy identifier's token);
- `original position` = the leaf at the same place in the real file (may be null);
- `original file` = the real file;
- the offset used for position queries is the caret offset in the copy.

The copy is non-physical; contributors must not modify it. For injected-language fragments the
copy is made in the top-level file and mapped.

`CI-CMP-010` MUST: the pipeline of 3.1 and 3.2, in particular that completion works in code
that does not currently parse, and that no contributor mutates the live buffer. Acceptance: in
`foo(|` (unclosed call) the member completion still lists the right candidates.

### 3.3 Parameters seen by contributors

`position`, `original position`, `original file`, `completion type`, `offset`, `invocation count`,
`is auto-popup` (= count 0), `is extended completion` (BASIC with count >= 2), the editor, the
process (for user data and abort), and flags *testing mode* and *complete only not imported*.

### 3.4 Running contributors

- Contributors are registered per language (and "any"), with *order* relative to others
  (`first`, `last`, `before X`, `after X`). The list for a language includes its base languages'
  contributors. A contributor may declare dumb-awareness; in dumb mode only dumb-aware ones run
  (the footer shows `Analyzing project...`, and an empty result in dumb mode reads
  `No suggestions; analyzing project`).
- Each contributor receives the parameters and a **result set**. It runs inside a read action on
  a background thread, must call the cancellation check regularly, and may add items one by one or
  in batches.
- The platform contributes defaults: a *default* contributor (reference variants: `getVariants`
  of the reference at the caret, filtered by the prefix; order `last`), a *word* contributor
  (words of the current file, order `last`, see 3.8), file-path completion, live-template items.
- A contributor can **stop** others (use sparingly), and can run the remaining contributors
  itself to filter their output.
- The first item arrives -> the session waits **up to max(300 ms, 2000 ms minus elapsed)** for
  completion to finish (the *synchronous phase*). If it finishes in time the result is
  processed at once (section 3.5, 3.6). Otherwise the lookup is shown with the items so far and
  keeps loading in the background (phase *background calculation*); the list is updated as items
  arrive and shows a `loading` indicator at the bottom while running.

### 3.5 Result set and matching of items

A result set wraps: the sink (the lookup), the **prefix matcher**, the **relevance sorter**, the
originating contributor and the parameters. Operations:

- *add element*: matches the element against the prefix matcher (every lookup string of the
  element is tried, section 6); items that do not match are dropped; the rest receive the
  sorter and matcher for later use and are passed on;
- *add all elements*: as a batch (sorted and displayed together; use it when relevance cannot be
  produced in order, see 7.4);
- *with prefix matcher / with prefix*: a derived result set using another matcher (e.g. one that
  ignores the part of a path before the last slash);
- *case insensitive*: derived result set that ignores the *Match case* setting;
- *with relevance sorter*: derived result set with a different sorter (7.1);
- *restart completion on prefix change*: when the user's prefix changes and satisfies a given
  pattern, the session is **re-run** instead of merely re-filtering (used by contributors whose
  candidate set depends on the exact prefix; also used when "not all items are shown");
- *restart when nothing matches*: re-run when the filter leaves zero items;
- *add lookup advertisement*: text in the lookup footer (section 9.4);
- *stop here*, *is stopped*.

### 3.6 Result processing after the first sync phase

For an **explicit** invocation (count >= 1) and a finished sync phase:

1. No items: show an information hint above the caret (`No suggestions`, or the text a
   contributor provides for the empty case; in dumb mode with the suffix `; analyzing project`).
   Pressing the completion shortcut again while the hint shows continues with the next
   invocation count (section 11).
2. Exactly one item and auto-insert is allowed (see 3.7): insert it and finish, without a popup.
3. Otherwise show the lookup (focused, section 8.1).

For an **auto-popup** invocation: if there are no items nothing is shown and the session ends;
otherwise the lookup is shown *unfocused* or *semi-focused* (8.1).

### 3.7 Single-item auto-insert decision

When `Automatically insert single suggestions` is on for the type, and the invocation is
explicit, the single item is inserted (no popup) unless one of these holds (then the popup is
shown):
1. the item's *auto-completion policy* is `never` (it is a "do not auto-insert" item); policy
   `always` inserts regardless of the other rules; policy `give chance to overwrite` inserts only
   when the caret is not inside an identifier (see rule 4);
2. the item does not match as a *start* match of the typed prefix (it matched in the middle);
3. the setting for this completion type is off;
4. the caret is **inside** an identifier (the identifier end differs from the selection end):
   the user may intend to overwrite, so the popup is shown;
5. a contributor's *auto-completion possibility* hook says "show lookup" (a contributor may also
   force insertion).

### 3.8 Word completion

On explicit BASIC invocation (never auto-popup) where nothing else matches in a place where
plain words make sense (plain text files, comments, strings, dumb mode for languages that opt
in), the *word contributor* offers the identifiers/words that occur elsewhere in the same file
(case-insensitive match on the word prefix, whole-word units), plus values of other string
literals when inside a string literal. Items show a *word* icon. The set excludes words the other
contributors already produced.

### 3.9 Cancellation and restarts

Typing while the session runs updates the prefix and filters existing items; if the typed
characters invalidate the context (non-identifier character, selection change, caret jump) the
session is cancelled. Pressing `Esc`, clicking elsewhere, moving the caret out of the prefix
area, or editing text outside hides the lookup. Contributors are cancelled at their next
cancellation check; results of a cancelled run are discarded.

`CI-CMP-020` MUST: contributor registration with language, order and dumb-awareness; the result
set operations of 3.5 (add, add all, with prefix matcher, case insensitive, with relevance sorter,
restart on prefix change, restart when nothing matches, advertisement, stop).
Acceptance: unit tests for each operation with a fake contributor.

`CI-CMP-021` MUST: the sync phase and background continuation of 3.4. Acceptance: a contributor
that takes 5 s still shows the lookup after about 2 s with partial items and a loading
indicator; items added later appear without moving the selection.

`CI-CMP-022` MUST: result processing 3.6 and the single-item decision 3.7. Acceptance: S-CMP-3.

`CI-CMP-023` MUST: word completion 3.8 on explicit invocation only. Acceptance: in a `.txt`
file typing `Hel` then `Ctrl+Space` offers `Hello` from earlier text; auto-popup does not.

## 4. Auto-popup rules

The auto-popup starts a session with invocation count 0 without the user pressing a key.

- **Trigger characters.** After the *typed handler* inserts a character: a letter, digit or
  underscore starts the auto-popup (unless a lookup is already open, in which case the character
  just extends the prefix). Languages add their own triggers via the *typed handler* hook: Java
  and Kotlin trigger after `.` (member access) and after `new `; Java also after `@` in doc
  comments, `"` for class names in strings where applicable; the exact list belongs to the language
  specs.
- **Timing.** There is no artificial delay: the session starts as soon as the document is
  committed to the syntax tree after the keystroke.
- **Cancellation.** Any other user activity (typing, an action, mouse activity not caused by a
  hint) cancels a pending auto-popup start.
- **Gates.** Only if the setting `Show suggestions as you type` is on (or the editor is flagged
  "always auto-popup"), power-save mode is off, the editor is not an inactive one, the file is
  not "large" for the language (a size gate disables auto-popup in huge files), and the
  *completion confidence* of the position allows it.
- **Completion confidence** (extension, per language): a rule returns *yes* (skip auto-popup),
  *no* or *unsure*. Typical skips: inside comments (except doc tags), after a number literal,
  after a keyword that is followed by a name the user defines (`class Foo`), inside the declaration
  name position, in string literals that do not contain references.
- **No results** for an auto-popup: nothing is shown; the session remains *tracking* so more typing
  can retry (up to a few characters).
- **One lookup at a time.** Starting a new auto-popup closes the current session.

`CI-CMP-030` MUST: triggers, timing, gates and confidence of this section. Acceptance: S-CMP-1,
and a test that typing `1.` after a number does not open member completion.

`CI-CMP-031` MUST: no popup while typing identifiers in a declaration-name position
(`class Fo|`, `int co|`) unless the language says otherwise. Acceptance: typing `int co` does not
pop up completion.

## 5. Lookup elements and presentation

### 5.1 Lookup element contract

| Part | Meaning |
|---|---|
| lookup string | the primary string inserted and matched |
| all lookup strings | extra strings matched against the prefix (aliases); the primary string is included |
| object | the thing the item stands for (a symbol, a string); items with equal objects are duplicates and are merged (keep the first) |
| PSI target | optional: the element used for documentation, deprecation, navigation |
| presentation | how the row looks (5.2) |
| insert handler | what happens after the lookup string was inserted (section 10) |
| case sensitivity | whether the *Match case* setting applies to this item |
| auto-completion policy | `settings dependent` (default), `never`, `always`, `give chance to overwrite` |
| validity | the item may become invalid after PSI changes; invalid items are dropped |
| worth showing in auto-popup | items may refuse to show in auto-popup (rare) |
| requires committed documents | the handler needs a committed tree |

Decorations: *priority* (a number added by wrapping with a prioritised element; higher is
better), *grouping* (an integer to group items together), *explicit proximity*, *tail type*
(what to append: a space, a semicolon, parentheses; see 10), *auto-import* flags.

### 5.2 Presentation (what the row shows)

Left to right, in the popup's single row:

1. **icon** (the symbol kind: class, interface, enum, method, field, variable, parameter,
   keyword, template, word, file, package, annotation...), optionally with a visibility overlay
   (registry switch, default off);
2. **item text** with the *matched characters of the prefix* drawn **bold** (and in the selection
   colour on the selected row); optional text decorations: bold, italic, underline, strike-out
   (strike-out = deprecated), foreground colour; named *decorated ranges* inside the text;
3. **tail text**: grey text right after the item text: for methods the parameter list, for classes
   `(package.name)`; fragments can be grey or normal and italic;
4. **type text**: right-aligned, grey (or normal): the member's type, e.g. `String`, with an optional
   *type icon*; may be right-aligned icon;
5. optional **shortcut or advertisement** marks for special rows.

Rows render lazily: a cheap presentation is computed when the item is added; an expensive
renderer may refine it later off the UI thread.

Size: the list shows at most **11** rows before scrolling (setting `max lookup list height`,
minimum 5); maximum width **500 px**; the popup follows the caret, below the line (or above when
there is no room); it keeps its position while the prefix grows.

`CI-CMP-040` MUST: the element contract (5.1) and presentation parts (5.2) including bold
matched characters, grey tail text, right-aligned type text, strike-out for deprecated items.
Acceptance: screenshot of S-CMP-1.

`CI-CMP-041` MUST: de-duplication of items with equal objects (keep first). Acceptance: two
contributors adding the same method yield one row.

`CI-CMP-042` SHOULD: lazy expensive rendering, 11-row height, 500 px width. Acceptance: with 1000
items, scrolling does not block (rendering is incremental).

## 6. Prefix matching

The prefix matcher decides which items match the typed prefix and where the matched fragments are
(for bold drawing and for ranking). It is the **camel-hump matcher** and is shared with
navigation, speed search and the rest of the IDE.

### 6.1 Rules

Let the *pattern* be the prefix (when middle matching is on, the pattern gets an implicit
leading wildcard `*`, and each `.` is followed by an implicit wildcard word boundary).

1. **Words.** A name is split into *words* at: transitions from lowercase to uppercase
   (`fooBar` -> `foo`, `Bar`), runs of uppercase followed by lowercase (`HTMLParser` ->
   `HTML`, `Parser`), digits, and the separators `_`, `-`, `.`, `$`, spaces and path
   separators.
2. **Uppercase in the pattern** starts a new word: an uppercase pattern character must match an
   uppercase letter of the name or the first character of a word (`NPE` matches
   `NullPointerException`; `fB` matches `fooBar` and `fooBarBaz`). A pattern that is *entirely*
   uppercase (no lowercase letter after a non-wildcard uppercase) may additionally match
   lowercase letters when the case setting does not demand exact case.
3. **Lowercase in the pattern** continues the current word or, as *lowercase humps*, begins the
   next word: `cmh` may match `camelMatchHump` (`c`, `m`, `h` word starts). Lowercase humps work
   across words unless a *hard separator* (a path separator, for file completion) lies between.
   The first pattern character must match the first character of the name or (with middle
   matching) the first character of any word.
4. **Wildcard** `*` in the pattern matches any run of characters; a space is also treated as a
   boundary that can start a new word.
5. **Case.** By the setting `Match case`:
   - *none*: case is ignored entirely;
   - *first letter only* (default): the first letter of the pattern (wildcards at the start are
     skipped) must have the same case as the **first character of the name** (a lowercase
     pattern cannot select a name that starts uppercase and vice versa; this is how `string`
     and `String` are kept apart); all other pattern letters match case-insensitively. The test
     compares with the first character of the *whole name*, even when the match itself lands in
     the middle of the name. A pattern or name whose relevant first character has no case (a digit,
     `_`) imposes no condition. For names that start with underscores the first letter after the
     underscores is used in the completion-specific check;
   - *all letters*: every pattern letter must agree in case.
6. **Start match vs middle match.** A match is a *start match* if the first matched fragment
   begins at the start of the name (leading underscores skipped). Everything else is a
   *middle match* (only available when middle matching is on).
7. **Fragments.** The result is the list of matched ranges `(start, end)` (merged when adjacent).
8. **Degree.** A numeric score of match quality: higher for start matches, matches on word
   boundaries, longer consecutive fragments, fewer fragments, exact-case matches and shorter
   names. Used for ordering (7.2).
9. **All lookup strings.** An item matches if *any* of its lookup strings matches; the best
   score among the matching strings counts.
10. **Typed prefix > 100 characters:** fall back to plain substring matching.

### 6.2 Examples (acceptance table, default settings: middle matching on, `First letter only`)

| Pattern | Name | Matches? | Start match |
|---|---|---|---|
| `NPE` | `NullPointerException` | yes | yes |
| `nPE` | `NullPointerException` | no (`n` lowercase, name starts uppercase) | |
| `cmh` | `camelMatchHump` | yes | yes |
| `fB` | `fooBar` | yes | yes |
| `foob` | `fooBar` | yes | yes |
| `bar` | `fooBar` | yes (word `Bar`, case-insensitive) | no |
| `Bar` | `fooBar` | no (`B` uppercase, name starts lowercase) | |
| `ex` | `getExample` | yes (middle) | no |
| `Fo` | `foo` | no | |
| `fo` | `Foo` | no | |
| `a` | `_a` | yes (leading underscore skipped) | yes |

With `Match case` off every "no" caused by case becomes "yes". The implementation must agree with
the reference matcher on a **golden table** of at least 200 pattern/name pairs, generated from the
IntelliJ matcher and committed as data; the table above is its seed.

### 6.3 Prefix and replacement ranges

- The *prefix* is computed from the start of the identifier to the caret using the language's
  identifier characters (or the reference's range start). Contributors that match other kinds of
  text (file paths, CSS values) supply their own prefix.
- Characters typed afterwards extend the prefix by *additional prefix*; the lookup re-filters
  existing items; if a *restart pattern* matched, the session re-runs.
- `Backspace` shrinks the additional prefix; when it would shrink below the original prefix start
  the lookup hides (for explicit) or re-queries.

`CI-CMP-050` MUST: the matcher of 6.1 with fragments and degree, case settings, start/middle
match, and a golden-table test against the reference matcher (at least 200 pairs, kept in the
repository as data). Acceptance: all golden pairs agree on match yes/no, start match, and the
fragments.

`CI-CMP-051` MUST: the same matcher serves completion, Search Everywhere, Go to symbol and the
speed search in popups. Acceptance: one implementation crate used by all callers.

`CI-CMP-052` MUST: middle matching on by default; the registry-equivalent switch exists.
Acceptance: typing `ar` offers `fooBar` below start matches such as `array`.

## 7. Sorting and relevance

### 7.1 The classifier chain (exact order)

Items are ordered by a chain of **classifiers**; each classifier partitions the items into
ordered groups; the next classifier orders *within* each group; ties at the end keep the
**presentation order** (see 7.3). The default chain, from most to least significant:

| # | Id | Rule | Direction |
|---|---|---|---|
| 1 | `middleMatching` | start matches before middle matches | start first |
| 2 | `liftShorter` | an item whose lookup string is a **proper prefix of another item's lookup string** is lifted to sit immediately before the first longer item it prefixes (so `foo` appears just before `fooBar`, never after) | by lifting |
| 3 | `priority` | the *priority* number of a prioritised element (default 0) | higher first |
| 4 | `prefix` | best **matching degree** of any of the item's lookup strings (6.1.8) | higher first |
| 5 | `stats` | usage recency in this context (7.2) | more recent first |
| 6 | language weighers | zero or more weighers registered by the language and by plugins, each with an id and an *order* relative to the others (examples for Java: expected-type match, locality of declaration, "not deprecated" etc.) | per weigher |
| 7 | `explicitProximity` | proximity set explicitly by the producer | closer first |
| 8 | `proximity` | project-structure proximity of the item's target to the caret's file: opened in editor, same directory, same source root, same module, in resolve scope, directory type (production over test), SDK/library last | closer first |
| 9 | `grouping` | the *grouping* number of a prioritised element | smaller first |

A *weigher* is a function from (item, location) to a comparable value; its registration gives
an id and *before/after* constraints. Producers may build a derived sorter with their own
weighers inserted before or after a given id (`with relevance sorter`).

### 7.2 Statistics (the `stats` classifier)

The IDE remembers which item the user chose in which *context* (the producer supplies a context
string and a value string for each item: for Java, e.g. the expected type or qualifier type and the
member name). The weight of an item is `10000 - recency`, where *recency* is the number of
selections made in the same context after the last time this item was chosen; a recency beyond
10000, or an unknown item, has weight 0. Items with a positive weight are ordered by weight,
highest first, **but** items without history that already lead the order produced by the
later classifiers stay in front of every item with history (so a fresh, strongly matching item is
not pushed down by an old choice).

`CI-CMP-060` MUST: store (context, value, selection counter) per chosen item, persisted per user,
bounded in size. Acceptance: choose `substring` after `String s; s.` twice: next time `s.`
puts `substring` above alphabetically earlier items of equal match quality.

### 7.3 Presentation order and the rest

Before classification, items are sorted by their **initial presentation order**: start matches
then middle matches, each sorted by item text (natural order), then by length of tail text,
then by tail text, then by type text. This is the stable base order; the classifiers refine it.

### 7.4 Arranging the visible list (what the user sees first)

The final list is arranged as:

1. **top priority items** (items a contributor marks as always first) sorted by relevance;
2. **exact prefix matches**: items whose lookup string equals the typed prefix, case-exact first,
   then case-insensitive;
3. **frozen items**: the items that were already shown at the top in earlier arrangements of
   this session keep their positions (so the list does not jump under the user's eyes while
   more items arrive or the prefix changes slightly). Frozen items are re-computed whenever the
   prefix changes;
4. enough items by relevance to bring the preferred block to **5** items;
5. the currently selected item (if the user has not touched the selection);
6. the item to preselect;
7. the rest in relevance order.

If the user turned on `Sort suggestions alphabetically` the list is simply presentation order
(start matches then middle matches, alphabetical).

**Preselection** (the row selected when the lookup opens): the best *exact* match of the typed
prefix, else the most relevant item that is not marked "never preselect"; if the user already
moved the selection, the previously selected item is kept when still present (or an item with the
same presentation).

**Item limit:** at most **1000** items are kept; when the limit is reached the engine keeps
the top-priority items, exact matches, frozen items and the most relevant ones up to half the
limit, shows the footer advertisement `Type more characters to see all suggestions`, and
**restarts** completion on any further prefix change.

`CI-CMP-061` MUST: the classifier chain of 7.1 including lift-shorter and start-before-middle,
with an extension point for language weighers carrying ids and before/after constraints.
Acceptance: a golden test: given 12 items with known attributes the order equals the reference
order (data file kept in the repository).

`CI-CMP-062` MUST: presentation order 7.3 and arrangement 7.4 including frozen items and the
5-item preferred block. Acceptance: while items stream in, the top 5 rows never change position
once displayed (unless the prefix changed).

`CI-CMP-063` MUST: the 1000-item limit with the advertisement text and restart on prefix
change. Acceptance: completion on an empty prefix in a project with >5000 names shows 1000
items, the footer text, and a re-run when typing one more character.

`CI-CMP-064` SHOULD: statistics classifier 7.2. Acceptance: see CI-CMP-060.

`CI-CMP-065` SHOULD: relevance debugging: an action (default `Ctrl+Alt+Shift+W`) that dumps the
per-classifier weights of each visible item to the log and clipboard. Acceptance: output lists
`middleMatching`, `liftShorter`, `priority`, `prefix`, `stats`, ... per item.

## 8. Lookup behaviour and keyboard

### 8.1 Focus degrees

| Degree | When | Selection | `Enter` | `Tab` | Context keys (`.`, `,`, `;`, `=`, `space`, `:`, `(`) |
|---|---|---|---|---|---|
| **Focused** | explicit invocation; also auto-popup after `Down/Up`; or auto-popup when the setting `Insert selected suggestion by pressing space, dot, or other context-dependent keys` is on and the context says preselect | first (best) item preselected | inserts selected item | inserts and **replaces** the identifier tail | select the item, then type the key |
| **Semi-focused** | auto-popup when the context says "preselect the first suggestion" and the setting above is off | best item preselected and drawn in the non-focused selection style | inserts selected item | inserts and replaces | **do not** select: the lookup closes and the typed character is inserted normally (so `foo.` is not auto-completed to the first suggestion); `Up/Down` turns it focused |
| **Unfocused** | auto-popup in contexts where preselection is not wanted (context says no) | no row selected | works as in the editor (newline, indentation) | inserts the **top** item | no selection; lookup closes |

`Ctrl+Space` again, `Alt+Enter` (element actions), `Ctrl+Shift+Enter`, `Ctrl+.` (finish with dot)
are available in all degrees.

### 8.2 Keys while a lookup is open

- Letters, digits, `_`, `$` (identifier parts): extend the prefix (`ADD_TO_PREFIX`).
- Selection characters `.` `,` `;` `=` ` ` `:` `(`: finish the lookup by selecting the current
  item **if the lookup is focused**, then type the character (so `.` after a chosen item
  continues the chain). If not focused: hide the lookup and type the character.
- `-`: extends the prefix if some item matches after appending it, otherwise hides the lookup
  (CSS-like names).
- Any other character: hides the lookup and is typed.
- `Up`/`Down`: move the selection (wrap at both ends); `PageUp`/`PageDown` and `Home`/`End`
  behave as in any list popup; `Ctrl+Down` / `Ctrl+Up` move the **editor caret** down and up
  while the lookup stays open (advertised in the footer, see 9.4).
- `Enter`: normal selection (see table). `Tab`: replace selection. `Ctrl+Shift+Enter`: select
  then complete the statement. `Ctrl+.`: select then insert a dot.
- `Backspace`: shortens the prefix (8.3). `Delete` and other edits: hide.
- `Esc`: closes the lookup (does not consume the key for other handlers).
- `Ctrl+Space` (basic again): re-invoke with count+1 (section 11).
- Mouse: hover selects (hover highlight); click selects and finishes; double click as click.
- `Ctrl+Q`, `Ctrl+Shift+I`, `F1`: documentation and definition of the selected item shown beside
  the lookup.

### 8.3 Prefix edits

Typing extends the additional prefix; the lookup filters its items and re-arranges. If *no* item
matches and the restart rule does not apply, the lookup closes (explicit: hint `No suggestions`
the first time). Backspacing below the start of the prefix closes it (explicit) or restarts
completion (auto-popup). When the lookup contains items from an *incomplete* source (the 1000 limit,
a language server marked "incomplete"), prefix changes re-run completion.

### 8.4 Selected item and caret

Moving through the list **never** modifies the document. Inserting happens only on a selection
key, a click or a single-item auto-insert.

`CI-CMP-070` MUST: focus degrees and the key table of 8.1/8.2 exactly. Acceptance: S-CMP-1,
S-CMP-2 and a key-script test.

`CI-CMP-071` MUST: the character filter of 8.2 (identifier parts extend; the seven context keys
select if focused; `-` conditional; others hide). Acceptance: type `foo.ba` in a lookup with
`bar`, press `.`: focused lookup inserts `bar.`, semi-focused leaves `foo.ba.`.

`CI-CMP-072` MUST: Tab replaces; Enter inserts. Acceptance: text `Str|ing` (caret after `Str`),
choose `String`: Enter turns `Str` into `String` and keeps the suffix, giving `Stringing`; Tab
also removes the suffix, giving `String`. Test both.

`CI-CMP-073` MUST: Up/Down wrap-around and preselection rules of 7.4. Acceptance: key script.

## 9. The popup chrome

### 9.1 Layout

Rows as in 5.2; a thin scrollbar; a **footer** with one advertisement line (9.4) and, while
loading, an animated indicator; no title. The selected row uses the list selection colours; a
non-focused (semi-focused) selection uses a lighter variant. The popup is not focus-stealing:
keyboard focus remains in the editor, keys are intercepted by the lookup.

### 9.2 Quick documentation next to the lookup

When *Show the documentation popup in N ms* is on (or `Ctrl+Q` is pressed) the documentation of
the selected item appears to the side (right or left, wherever there is room) after the delay
and follows the selection. Content from the doc provider of the item's target. (Specified in the
documentation spec.)

### 9.3 Inline hint "`Press ... to ...`" advertisements: see 9.4.

### 9.4 Footer advertisements

The footer shows **one** line chosen from the registered advertisements. Registration comes from
contributors (`add lookup advertisement`), from the platform defaults, and from the engine
(overflow). A feature-based advertisement (one that teaches a shortcut) is shown only while the
user has used that feature fewer times than a threshold (the *feature usage tracker*; Archipel:
3 uses, per user, persisted).

Platform defaults, in the order they are considered (the first eligible is shown; the rest are
available for rotation on later sessions):

| Condition | Text |
|---|---|
| dumb mode | `Analyzing project...` with a spinner (in light mode the second line is `Light mode`) |
| otherwise | `Press <Enter> to insert, <Tab> to replace` (shortcuts from the keymap) |
| identifier tail would be replaced by Tab | `Use <Tab> to overwrite the current identifier with the chosen variant` |
| auto-popup and preselect and the "insert by chars" setting off | `Press <Ctrl+.> to choose the selected (or first) suggestion and insert a dot afterwards` |
| auto-popup | `<Ctrl+Down> and <Ctrl+Up> will move caret down and up in the editor` |
| invocation 1 BASIC in Java-like languages | `Press <Ctrl+Space> to see non-imported classes` |
| overflow (7.4) | `Type more characters to see all suggestions` |
| documentation-in-lookup tip | `Did you know that Quick Documentation View (<shortcut>) works in completion lookups as well?` |
| definition tip | `Did you know that Quick Definition View (<shortcut>) works in completion lookups as well?` |
| smart-enter tip | `Use <Ctrl+Shift+Enter> to syntactically correct your code after completing (balance parentheses etc.)` |
| context keys tip | `Dot, space and some other keys will also close this lookup and be inserted into editor` |

`CI-CMP-080` MUST: the popup layout 9.1 and the footer line with at least the first four texts of
9.4 (shortcuts taken from the active keymap). Acceptance: screenshot of S-CMP-1 shows the footer
`Press Enter to insert, Tab to replace`.

`CI-CMP-081` SHOULD: feature-usage gating of tip advertisements and rotation. Acceptance:
after the user chose with `Tab` three times, the "use Tab" tip stops appearing.

`CI-CMP-082` SHOULD: documentation popup beside the lookup. Acceptance: `Ctrl+Q` shows docs.

## 10. Insertion

### 10.1 Sequence (on a selection key)

1. Close the lookup (but keep the session data); remember the selected item, the *completion
   char* (the character that selected it: `\n` for Enter, `\t` for Tab, `\r`-class for
   Ctrl+Shift+Enter, the typed character for context keys, or "none" for auto-insert).
2. Open **one command** (named `Autocompletion` for single-item auto-insert and for auto-popup,
   otherwise named after the invoked action) and a write action; the whole insertion is one undo
   step.
3. Move the caret to the end of the typed prefix. If the completion char is `Tab` and text
   remains between the caret and the *replacement end*, **delete** that text.
4. The default insertion replaces the prefix (from `start`) with the item's lookup string.
   The offset map now holds: `start`, `tail` (end of the inserted string), `selection end`.
5. If the item requires committed documents, commit the document to the syntax tree.
6. Call the item's **insert handler** with the insertion context: editor, file, start offset, tail
   offset, completion char, the list of all items, and the offset map. Handlers may edit the
   document (add parentheses, imports, qualifiers, type arguments), start templates, move the
   caret, add offsets to the map, and request that the completion char not be re-typed.
7. Process postponed reformatting.
8. If the completion char is a *typed character* (not Enter/Tab/none) and the handler did not
   consume it, **re-type it** at the caret through the normal typed-handler chain (so `.`,
   `(`, `;`, `=`, space behave as if typed, including auto-pairs and the next auto-popup).
   For `Ctrl+Shift+Enter` run the language's *smart enter / complete statement* processors after
   insertion. For Enter and Tab nothing is typed.
9. Scroll the caret into view; update the statistics (7.2); fire the *item selected* event for
   features that listen (parameter info after `(`, auto-popup after `.`).
10. Undo of the whole operation restores the document and caret.

### 10.2 Standard tail behaviour (insert handlers)

| Item kind | After inserting the name |
|---|---|
| method | if `Insert parentheses automatically when applicable` is on: `()` and the caret between them when the method has parameters, after them otherwise; then parameter info is requested; a `;` or tail type is added for statement-like context only on `Ctrl+Shift+Enter` |
| class used as type | import if needed (10.3), type arguments `<>` when the language wants them, constructor parentheses after `new` |
| variable, field | the name only (a qualifier is added if it would otherwise not resolve, language decision) |
| keyword | the keyword plus language-specific trailing space or braces template |
| live template item | the template expands (see the templates spec) |
| file path | the path component and `/` for directories, then auto-popup again |
| LSP item | `textEdit` or `insertText`; `additionalTextEdits` (imports) applied in the same command; snippet format starts a template session |

### 10.3 Auto-import on completion

Selecting an item for a class (or static member) that is not imported adds the import as part of
the insertion (same command, same undo step) using the same import placement rules as
`05-auto-import.md` 9. Rules:
- an import is added only when the language's setting allows (`Insert imports on completion`);
  for classes the import is added; for names already visible nothing is added;
- candidates excluded by the *Exclude from auto-import and completion* list are not offered;
- static members appear in completion only for classes listed in *Include auto-import of static
  members in completion* (or when the user invoked the second/third completion);
- if two classes with the same short name exist, both are offered and the row's tail text shows
  the package so the user can choose (`List  (java.util)`, `List  (java.awt)`).

### 10.4 Replacement end

With `Enter` the text after the caret is kept; with `Tab` the identifier (or the reference
range, if a reference exists at the caret) to its end is replaced. The language may set the
identifier end explicitly in `before completion`.

`CI-CMP-090` MUST: the insertion sequence 10.1 (one command, one undo step, prefix replaced, Tab
deletes tail, completion char re-typed for typed characters, nothing re-typed for Enter/Tab).
Acceptance: S-CMP-2, S-CMP-4.

`CI-CMP-091` MUST: the standard handlers for methods (parentheses, caret placement), classes
(import + type args + constructor parentheses after `new`), keywords. Acceptance:
completing `substring` offers `substring()` with the caret inside when the method has
parameters (`substring(|)`) and `length()|` when none; completing a class name adds the
import (S-CMP-5).

`CI-CMP-092` MUST: auto-import on completion 10.3 with a package shown for duplicates and the
exclude/include lists. Acceptance: S-CMP-5.

`CI-CMP-093` MUST: `Autocompletion` command name for auto-inserted single items. Acceptance:
the Undo menu reads `Undo Autocompletion`.

`CI-CMP-094` SHOULD: `Ctrl+Shift+Enter` completes the statement after insertion (balancing
parentheses, adding `;` and braces). Acceptance: `list.add(` + choose... see S-CMP-6.

## 11. "Press X again to see more" and invocation counts

### 11.1 Invocation count rules

- The first explicit invocation has count 1; **while a lookup of the same completion type is
  showing at the same caret position**, invoking the same action again starts a new session with
  count + 1 (the lookup is replaced; the user keeps typing position and prefix). Invocation of
  the same action when no lookup is showing resets to 1.
- Count 2 or more with `BASIC` is *extended completion*; with `SMART` it is *chain*/deeper
  completion defined by the language.
- Pressing the shortcut again while the **`No suggestions` hint** of an empty explicit session is
  still showing also continues to count 2 (so an empty first level is followed by the extended
  level without the user retyping anything).
- The promoted count is `max(previous + 1, 2)`.
- Invoking a different completion type (for example `SMART` while a `BASIC` lookup shows)
  starts at count 1 for that type.

### 11.2 What higher counts add (framework contract; the language decides content)

- Contributors read the count: count 1 shows what is **visible without extra imports and without
  expensive searches**; count 2 adds *more* (examples for Java: classes that are not imported, with
  import on insertion; static members that are not imported; inherited private members; all
  names from other modules); count 3+ may add everything (all symbols from the index).
- Contributors that have more to offer at the next count register the footer advertisement
  `Press <Ctrl+Space> to see non-imported classes` (Java wording; other languages use their
  own word for the content) via the usual advertisement mechanism, subject to feature-usage
  gating (9.4).
- The engine passes the same offsets and prefix; items from count 1 that are still valid stay
  visible in the same order where possible (frozen items), so the user sees the new items appear
  below.
- For `SMART` the same pattern: first invocation: items whose type fits; second: items that need
  one method call to fit (chains), advertised by `Press <Ctrl+Shift+Space> again to see
  chain suggestions` style text.

`CI-CMP-100` MUST: the invocation-count semantics 11.1, exposed to contributors as an integer and
as `is extended completion`. Acceptance: pressing `Ctrl+Space` three times in a row shows counts
1, 2, 3 to a test contributor; after `Esc` it restarts at 1.

`CI-CMP-101` MUST: for Java/Kotlin, count 2 adds non-imported classes whose insertion adds the
import (10.3), and the footer text of 11.2 is shown at count 1 (while feature-gating allows).
Acceptance: S-CMP-5.

`CI-CMP-102` SHOULD: SMART completion with expected types at counts 1 and 2 (needs the type
engine; tier G3+). Acceptance: `String s = |` + Ctrl+Shift+Space lists only String-typed
candidates; second press adds chained calls.

## 12. Dumb mode, errors and cancellation behaviour

- In dumb mode only dumb-aware contributors run. An empty result shows the hint
  `No suggestions; analyzing project`; if a non-dumb-aware part of the engine raises
  an index-not-ready error during an *explicit* invocation, a notification `Code completion is
  not available here while indexes are being built` is shown and nothing else happens. Items
  that are present show the footer text `Analyzing project...`.
- An exception in one contributor is logged and ignored; the others continue.
- Cancellation is cooperative: contributors must be cancellable at every loop; cancellation
  during the sync phase falls back to showing whatever was gathered.
- If the lookup was shown and an exception occurs on insertion, the document state is restored
  (the insertion is within one command; a failed handler aborts the command).

`CI-CMP-110` MUST: dumb-mode rules and texts above. Acceptance: with indexing running,
`Ctrl+Space` in a Java file shows only dumb-aware items and the footer line `Analyzing project...`;
an empty result reads `No suggestions; analyzing project`.

`CI-CMP-111` MUST: contributor failures never break the lookup. Acceptance: a contributor that
panics yields the other items.

## 13. Built-in contributors for Archipel (language-independent baseline)

For languages that have only a tree-sitter grammar (tier G0-G1, see
`docs/analysis/code-intelligence-spec.md` Part 3): keywords valid at the position (from the
grammar's keyword tokens, filtered by a per-language "keyword context" rule), local names in
scope, same-file declarations, words from the file (3.8), file paths in strings, live-template
abbreviations, postfix templates. The LSP contributor adds the server's items.

**LSP mapping** (`textDocument/completion`):

| LSP field | Engine concept |
|---|---|
| `label` / `labelDetails` | item text / tail text (`detail` of labelDetails) and type text |
| `filterText` | additional lookup string (matching) |
| `sortText` | tie-break key after the classifiers (presentation order uses `sortText` when present) |
| `kind` | icon; `deprecated` / `tags: [Deprecated]` | strike-out |
| `preselect` | priority high and eligible for preselection |
| `isIncomplete` on the response | "incomplete source": re-run on prefix change |
| `textEdit` / `insertText` + `insertTextFormat` | insertion text (snippet format starts a template session with tab stops) |
| `additionalTextEdits` | applied in the same command as the insertion (auto-import) |
| `commitCharacters` | additional selection keys for this item |
| `command` | executed after insertion |
| `resolve` | lazy docs and additional edits when the item is selected or inserted |
Items from a server and native contributors are merged: duplicates (same lookup string and same
kind) keep the native item.

`CI-CMP-120` MUST: the LSP mapping above is the only place that knows LSP types; the engine sees
only lookup elements. Acceptance: swapping the LSP provider for a fake does not change the
engine tests.

`CI-CMP-121` SHOULD: baseline contributors of the paragraph before the table. Acceptance: in a
language without native resolve, completing a word in a comment/string or a local name works.

## 14. UX acceptance scenarios

**S-CMP-1, auto-popup while typing (Java, default settings).** In a method body of an indexed
project the user types `str`. As soon as the document is committed (no extra delay) a lookup
appears below the caret with at most 11 rows: start matches first, such as `String` (class icon,
tail text `(java.lang)`) and `StringBuilder`; then middle matches (members whose names contain
`str`). Matched characters are bold. The lookup is semi-focused: the first row is selected with
the lighter selection, the footer reads `Press Enter to insert, Tab to replace`. Typing `i` narrows
the list. `Enter` inserts `String`, the caret is after it, Undo restores `str`.

**S-CMP-2, Tab versus Enter.** Text `Str|ing` with the caret after `Str`, explicit `Ctrl+Space`,
choose `String`: Enter -> `Stringing`; undo; Tab -> `String` (the suffix `ing` replaced).

**S-CMP-3, single suggestion.** `Ctrl+Space` on `System.out.printl|` with a single start match
`println` (and its overload set merged): with `Automatically insert single suggestions` on, the
popup does not appear: `println()` is inserted with the caret inside the parentheses when the
chosen overload has parameters; if two overloads are both offered the popup appears.

**S-CMP-4, context key in a focused lookup.** `list.` -> auto-popup (semi-focused). `Down` to
`add` (now focused); type `(`: the lookup selects `add` and types `(`: result `list.add(` and
parameter info appears. In a semi-focused lookup (just opened) typing `(` simply closes the lookup
and types `(`.

**S-CMP-5, second invocation, non-imported class.** `Ctrl+Space` on `ArrayLi|` in a file without
imports: no `ArrayList` at count 1 if the language only offers visible classes (the footer reads
`Press Ctrl+Space to see non-imported classes`); pressing `Ctrl+Space` again shows `ArrayList
(java.util)` and `ArrayList (...)` variants with package tail text; choose one: the text becomes
`ArrayList` and `import java.util.ArrayList;` is added in the same undo step.

**S-CMP-6, complete statement.** `Ctrl+Shift+Enter` on `foo.bar` + selected method completion
inserts the method with parentheses, then runs complete-statement: result `foo.bar();` with the
caret on the next line or before `;` per the language's smart-enter.

**S-CMP-7, limit.** On an empty prefix in a huge project: 1000 items, footer `Type more characters
to see all suggestions`; typing one character re-runs completion and the footer disappears.

**S-CMP-8, camel humps.** Prefix `NPE` lists `NullPointerException` first; prefix `fB` lists
`fooBar`; prefix `ex` lists `exit` before `getExample` (start before middle match).

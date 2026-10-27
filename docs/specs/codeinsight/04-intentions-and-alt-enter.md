# 04. Quick fixes, intentions, `ModCommand`, and the Alt+Enter popup

Status: specification, language independent. Requirement prefixes `CI-FIX` (action and command
model) and `CI-ALT` (popup assembly, ordering, bulb, keyboard).
Priorities: `MUST` / `SHOULD` / `LATER`.

This is the part of the code-insight framework users touch most. The ordering rules in
sections 4 and 5 are exact: implement them literally, because the sort order is what makes the
first row of the popup the right one.

ic: `platform/analysis-api/src/com/intellij/codeInsight/intention/` (`IntentionAction`,
`PriorityAction`, `CustomizableIntentionAction`, `IntentionActionWithOptions`),
`platform/analysis-api/src/com/intellij/modcommand/`,
`platform/lang-impl/src/com/intellij/codeInsight/daemon/impl/ShowIntentionsPass.java`,
`.../intention/impl/CachedIntentions.java`, `IntentionListStep.java`,
`IntentionActionWithTextCaching.java`, `IntentionGroup.java`,
`DefaultIntentionsOrderProvider.java`, `ShowIntentionActionsHandler.kt`,
`IntentionHintComponent.java`, `IntentionsUIImpl.java`,
`platform/analysis-impl/.../daemon/impl/HighlightInfo.java` (`IntentionActionDescriptor`),
`.../daemon/impl/actions/FixAllHighlightingProblems.java`,
`platform/lang-impl/src/com/intellij/codeInsight/intention/impl/preview/`.

## 1. Vocabulary

| Term | Meaning |
|---|---|
| **Quick fix** | An action attached to a *problem* (a HighlightInfo). It exists because something is wrong or suspicious at that spot. Two sources: error fixes (attached by the general pass or annotators) and inspection fixes (attached by inspection problems). |
| **Intention** | An action that is *context-dependent but not tied to a problem*: "Invert if condition", "Add braces". It is queried at the caret each time. |
| **Action family** | A stable string naming a *kind* of action ("Import class", "Create method from usage"). Settings enable/disable intentions by family; Fix-all groups by family. |
| **Action text** | The text of one concrete instance ("Create method 'foo'"). |
| **Availability** | A cheap predicate "does this action apply at this caret/selection in this file". |
| **Fix range** | The range of text inside which a fix is offered (`02-highlight-info.md` section 6). |
| **Row** | One entry of the popup. |

## 2. The action contracts

### 2.1 `IntentionAction` (classic form)

| Member | Contract |
|---|---|
| text | Row text. Shown only if the action is available. |
| family name | Stable family string; the key for the Settings | Editor | Intentions checkbox and for grouping. |
| is available(project, editor, file) | Must be fast (runs on the UI thread, many times per caret move). May use only data that is cheap to compute. Returns false to hide. |
| invoke(project, editor, file) | Performs the change. Called inside a *command* (so one undo step, named by the row text). Called inside a write action if `starts in write action` is true. |
| starts in write action | False when the action shows UI (dialogs, nested popups) and wants to take the write action itself. |
| preview | See section 8. |
| element to make writable | Optional: the element whose file must be writable (read-only files show a "make writable" prompt before invoke). |

Optional capabilities (separate small interfaces in IntelliJ; in Archipel traits or flags):

- **priority**: `TOP`, `HIGH`, `NORMAL`, `LOW`, `BOTTOM` (section 3).
- **icon**.
- **customization**: `show submenu` (default true), `selectable` (default true), `show icon`
  (default true), `show light bulb` (default true), `override bulb icon` (default false),
  `separator above` (default false), `tooltip text`, `ranges to highlight`.
- **options**: a list of sub-actions shown in the row's submenu (`IntentionActionWithOptions`),
  with a *combining policy*: either "only my options" or "my options plus the standard
  inspection options".
- **dumb-aware**: may run during indexing.
- **hint action**: has a "show hint" step in addition to being an intention (import fixes).
- **shortcut**: user-assigned keyboard shortcut for this intention.

### 2.2 `LocalQuickFix`

See `03-inspections.md` section 4.3. The editor never sees it directly: it is wrapped into an
intention-like action carrying the descriptor, so all of section 2.1 applies to it. Its text is
`name`, its family is `family name`.

### 2.3 `ModCommandAction` (new form, preferred)

A *pure* description of an action:

| Member | Contract |
|---|---|
| presentation(context) | Returns `null` when not applicable, else a `Presentation`: name (row text), priority, a list of *ranges to highlight* with a kind (affected, deleted, applicable-to), an optional icon, an optional *fix-all option* (a name and a predicate "does another action belong to my family"). Must not modify anything. |
| perform(context) | Returns a `ModCommand` (section 7). Must not modify anything; all edits are returned as data. |
| family name | As above. |
| preview(context) | Default: compute `perform(context)` and render the command's file edits as a diff. |
| available in batch | Default true. |

`ActionContext` = project, file, caret offset, selection range, element at the caret (optional).
Offsets are in the coordinate space of `file` (for injected fragments: the fragment).

Wrapper rules (bridge to the classic form):
- `is available` = `presentation(context) != null` (and the action is usable in dumb mode);
- text = presentation name; priority = presentation priority; icon = presentation icon;
- options = the fix-all option when the presentation has one (section 6.3);
- `invoke` = `perform` then *execute interactively* (section 7.3); if the file is a preview
  copy, execute against the copy instead;
- the wrapper never starts in a write action (the executor takes care).

`CI-FIX-001` MUST: the classic contract (2.1) with all capabilities of the list. Acceptance:
a test intention implementing priority, icon, options and `show submenu = false` shows
accordingly in the popup (CI-ALT).

`CI-FIX-002` MUST: the pure contract (2.3): presentation and perform never mutate files;
verified by a guard that fails the test when a document changes during `perform`.
Acceptance: an action that edits inside `perform` fails the guard test.

`CI-FIX-003` MUST: the wrapper rules of 2.3 (bridge both directions: a classic action can be
listed as a pure action returning "run this classic action"; a pure action is listed as an
intention). Acceptance: both forms appear in one popup with correct order.

`CI-FIX-004` MUST: an action's *family name* is stable across releases and its text is
free-form. Acceptance: renaming an action's text does not reset its Intentions-page checkbox.

### 2.4 Registering intentions and the Intentions settings page

- Intentions are registered per language (a language id, or "any") with: class/identity, a
  *category path* (for the settings tree), a description (HTML) and *before* / *after* example
  snippets (used by the settings page and as the fallback preview, section 8).
- **Settings | Editor | Intentions** (`SET-ED-023`): a tree by language then category; each
  intention has a checkbox; disabled intentions are *not queried at all* (they never appear and
  never show a bulb). Settings are stored by family name. Right pane: description, *Before* and
  *After* example editors.
- The popup row of an enabled intention offers, in its submenu, `Disable '<family>'` (or
  `Enable '<family>'` when it was disabled but is still shown because it is the current
  selection in the settings), `Edit intention settings`, and either `Assign shortcut…` or
  `Edit shortcut…` plus `Remove shortcut` when a shortcut exists.
- A *dump-aware* filter: in dumb mode only actions marked dumb-aware are queried.

`CI-FIX-010` MUST: registration with language and category; disabled-by-user intentions are
neither queried nor shown. Acceptance: untick "Flip ','" in the settings page; it vanishes
from the popup immediately (next popup).

`CI-FIX-011` SHOULD: the Intentions page with description and before/after samples.
Acceptance: selecting an intention shows its description and both samples.

`CI-FIX-012` SHOULD: submenu rows `Disable '<family>'`, `Edit intention settings` for every
plain intention. Acceptance: choosing `Disable '<family>'` unticks it in the page.

`CI-FIX-013` LATER: assign/edit/remove a keyboard shortcut for an intention, shown in the row.

## 3. Priority model

Five symbolic priorities map to weights (used by sorting, section 5):

| Priority | Weight |
|---|---|
| TOP | +20 |
| HIGH | +3 |
| NORMAL (default) | 0 |
| LOW | -3 |
| BOTTOM | -20 |

Marker shortcuts: *high-priority action* means HIGH, *low-priority action* means LOW.
For wrapped/delegating actions the priority of the *innermost non-delegating action that
declares one* counts. A pure action takes the priority from its presentation.

`CI-ALT-001` MUST: the five priorities and the weights above. Acceptance: table test.

## 4. Assembling the popup (exact algorithm)

Inputs: editor, file, caret offset `O`, selection. If `O` equals the file length and is
greater than 0, element lookups use `O - 1`.

The result is five *sets*: **error fixes**, **inspection fixes**, **intentions**,
**gutter actions**, **notification actions**. A row belongs to exactly one set. Rows are
*equal* (and therefore deduplicated within a set) when they have the same action class (or,
for the pure form, the same family identity) **and** the same text.

### 4.1 Collecting problem fixes (steps P1 to P6)

**P1. Infos near the caret.** From the document markup take all HighlightInfos with severity of
at least INFORMATION whose range overlaps the caret *line*. Split them:
- *at caret*: the info's range contains `O`, counting the end offset as contained
  (`start <= O <= end`);
- *other errors*: severity exactly ERROR, starts on the same line, and not at caret.

**P2. Fixes of infos at caret.** For each info at caret, for each registered fix descriptor of
that info:
  1. skip it if the fix's action is not usable in the current dumb/smart state;
  2. skip it unless the *fix range* contains `O` or `O` equals the fix range's end offset
     (the fix range defaults to the info's range);
  3. test availability of the action at `O`. For injected-language infos, test inside the
     injected editor/file mapping the offset; for others, in the host file;
  4. if available, add the descriptor to the *fixes* list.
  If **no** fix of this info is available but at least one was unavailable, take the first
  unavailable descriptor and add its **empty action** version (section 6.4) to the *fixes*
  list, provided the descriptor has a tool key that is not the plain "annotator" pseudo-tool and
  the key has a display name.

**P3. Remote errors.** If none of the infos at caret has severity ERROR, look at the *other
errors* in order of the markup (by start offset). For the first such info that contributes at
least one available fix (checked with P2 but *without* the fix-range test) whose text is not
already the text of a fix in the *fixes* list, add those fixes (still skipping text duplicates)
and stop looking at further infos. These fixes keep their own fix ranges, which do not contain
`O`, so they become `REMOTE_ERROR` rows.

**P4. The severity that decides the classification** (the *decider*). Take the infos at
caret plus, if P3 contributed fixes, that remote-error info. Ignore infos that are TODO comments
or the caret-driven "element under caret" highlights. The decider is the **highest severity**
among the remaining infos (if several share it they are merged into one composite whose
severity and type are that of the first). Its info type also selects the bulb style (section 9).
If there is no info at all, there is no decider.

**P5. Classify each fix** (let *decider* be the info of P4):
- if there is no decider: all fixes go to **error fixes** (the case of a remote error with no
  problem at the caret);
- otherwise for each fix descriptor:
  - if its own severity is ERROR (or unknown) **and** the decider's severity is ERROR: error
    fixes;
  - else if its own severity is INFORMATION: **intentions** (a fix whose problem is hidden: see
    the "do not show" contributor in 4.2);
  - else: **inspection fixes**.

**P6. Remember** the decider's info type and the caret offset used (the offset is also the
default location for the fix-offset rule in 10.3).

### 4.2 Collecting intentions and contributors (steps I1 to I5)

**I1. Languages.** The set of language ids of the file's view provider, of the injected file at
`O` (if any), and of every element on the parent chain of the element at `O`.

**I2. Intentions.** For every registered, user-enabled intention applicable to one of those
languages and usable in the current dumb state: test availability at `O`, preferring the
injected editor/file when the injected file is non-empty and the action is available there,
otherwise the host. For each available intention, build a descriptor whose **options** are:
`Disable '<family>'` (or `Enable`), `Edit intention settings`, and the shortcut rows. Skip it if
an equal action is already among the *fixes* of P2/P3 (an inspection fix must not also appear
as a plain intention). Add to **intentions**.

**I3. Contributors** (an extension point, each called in dumb-state-aware fashion and each
allowed to add to any of the five sets):
  - *do-not-show inspections*: for every enabled inspection whose effective level at this file
    is "do not show" (and that is applicable to the language and project type, not unfair, and
    only when not in dumb mode and the element is in the project or a scratch file): run it
    right now, synchronously, on the elements at the caret (parents of the element at `O` and
    parents of the element just left of `O`), and for every problem whose range contains `O`,
    add each of its fixes: to **error fixes** if the problem's highlight type is ERROR, else to
    **intentions**, with severity INFORMATION and the problem's range as the fix range, and with
    the inspection's display name as the row's tool name;
  - *cleanup*: if any descriptor in the inspection-fix set (or, failing that, in the error-fix
    set) belongs to an inspection flagged as a cleanup tool, add one row `Cleanup code` with the
    submenu `Edit cleanup profile settings` and `Cleanup code on…`, to the same set;
  - *gutter*: for the caret line, collect the click action, middle-click action, right-click
    action and popup-menu actions of each gutter icon whose range overlaps the line and is
    usable in the dumb state; add to **gutter actions** (deduplicated, original order); these
    rows are rendered with the text of the gutter action's presentation, separators from the
    action group, and are skipped when their text is empty;
  - *editor notifications*: file-level notification actions stored for this editor go to
    **notification actions**, with their own options.

**I4. Filters.** Apply every `IntentionActionFilter` (extension) to the four sets
(intentions, error fixes, inspection fixes, notifications): an action rejected by any filter
is removed.

**I5. Lazy fixes.** If some infos near the caret still compute lazy fixes, wait for them
(CI-HLI-051) before P2; the popup shows a progress "Searching for context actions…" only when
this takes noticeable time. After the popup is built, start computing lazy fixes of the next
infos in the background (nearest to the caret first).

### 4.3 Wrapping and the final list

Each descriptor is wrapped into a **row** with: cached text, optional icon, tool id, fix range,
and its **submenu options**. The submenu options are computed (section 6) and each option is
kept only if it is available at the caret (host or injected). A row *has a submenu* when the
action shows submenus (default true) and has at least one option that is a plain intention or an
error-fix option.

The final list (`getAllActions`) is built as follows:

1. concatenate: all **error fixes** (in the set's insertion order), then all **inspection
   fixes**, then each **intention** that is not already in the first two sets, then **gutter**
   rows, then **notification** rows;
2. drop rows not usable in the current dumb state;
3. sort with the order provider of the language at `O`. The *default* provider (section 5)
   is used unless the language registers its own.

`CI-ALT-010` MUST: P1 and P2 exactly, including inclusive end offset and fix-range test.
Acceptance: caret at the position right after `assertThat` (end of the identifier): the import
fix is offered; one character further right: offered only if its fix range covers it.

`CI-ALT-011` MUST: P3 remote errors: with an ERROR on the same line but away from the caret
and no error at the caret, its fixes are offered as `REMOTE_ERROR` rows, only for the first
error that contributes. Acceptance: scenario S-ALT-3.

`CI-ALT-012` MUST: P5 classification (decider rule). Acceptance: an inspection warning fix and
an error fix at the same caret: the error fix is in the error group only if the decider is an
ERROR; with only a warning at the caret both are in the inspection group.

`CI-ALT-013` MUST: I2 with language set, user-enabled filter, availability on host or injected
file, de-duplication against fixes. Acceptance: an inspection's quick fix that is also
registered as an intention appears once, in its problem group.

`CI-ALT-014` MUST: the do-not-show contributor of I3 for enabled inspections at level "do not
show". Acceptance: S-INS hidden-problem scenario (CI-HLI-101).

`CI-ALT-015` SHOULD: the cleanup row of I3. Acceptance: with an unused import (cleanup tool)
under the caret, the popup contains `Cleanup code` with a submenu of `Edit cleanup profile
settings` and `Cleanup code on…`.

`CI-ALT-016` SHOULD: the gutter and notification contributors. Acceptance: a line with a
run-gutter icon lists its actions after all code actions, separated.

`CI-ALT-017` MUST: filters of I4 and equality/deduplication by (class or family identity, text).
Acceptance: two identical import fixes registered by two infos produce one row.

`CI-ALT-018` MUST: the popup waits for lazy fixes under the caret (I5). Acceptance: see
CI-HLI-051.

## 5. Ordering: groups, weights, tie-breaks (exact)

### 5.1 Group of a row

Evaluated in this order, the first match wins:

| Condition | Group | Group priority |
|---|---|---|
| in the *error fixes* set and (the row's fix range is unknown or contains `O`) | `ERROR` | 30 |
| in the *error fixes* set otherwise (fix range elsewhere on the line) | `REMOTE_ERROR` | 10 |
| in the *inspection fixes* set | `INSPECTION` | 20 |
| in the *notification* set | `NOTIFICATION` | 7 |
| in the *gutter* set | `GUTTER` | -5 |
| is an empty action (the "tool name" row), reached only for empties in the intentions set | `EMPTY_ACTION` | -10 |
| is an advertisement action | `ADVERTISEMENT` | -10 |
| anything else (plain intentions) | `OTHER` | 0 |

Note the consequence: an *empty action* that lives in the error or inspection set sorts in that
group (because it is tested first) and uses its own LOW priority to sink to the bottom of the
group.

### 5.2 Weight and sort

`weight(row) = groupPriority(row) + priorityWeight(row)`, where `priorityWeight` is 0 for rows
without priority. The final order is:

1. by `weight` **descending**;
2. then by row comparison **ascending**: if the action defines its own ordering (suppress
   actions compare by their fix priority number, then by family name) that is used; else
   case-sensitive comparison of the row **text**. When the two compared rows differ in kind,
   the comparison that one of them defines is applied (mixed comparisons effectively compare
   family names against texts alphabetically);
3. then by **original position** (the sort is stable).

Separators are drawn above a row whose group differs from the previous row's group, and above a
row whose action asks for a separator.

**Worked weights** (group + priority):

| Row kind | Weight |
|---|---|
| error fix, TOP | 50 |
| inspection fix, TOP | 40 |
| error fix, HIGH | 33 |
| error fix, NORMAL | 30 |
| remote error fix, TOP | 30 (ties with a NORMAL error fix, then text decides) |
| inspection fix, HIGH | 23 |
| inspection fix, NORMAL | 20 |
| inspection fix, LOW (the empty "tool name" row) | 17 |
| remote error fix, NORMAL | 10 |
| notification, NORMAL | 7 |
| plain intention, HIGH | 3 |
| plain intention or BOTTOM inspection fix | 0 (sorted together by text) |
| gutter row | -5 |
| empty action in the intentions set, advertisement | -10 |

Read the table top to bottom for the visible order of a popup that has every kind. In
particular: a TOP inspection fix (40) outranks HIGH (33) and NORMAL (30) error fixes; a NORMAL
error fix outranks every inspection fix except TOP ones; every inspection fix outranks every
plain intention except BOTTOM ones; the tool-name row sinks below the other inspection fixes
but stays above all plain intentions.

`CI-ALT-020` MUST: the group table of 5.1, including the REMOTE_ERROR rule and the evaluation
order. Acceptance: table test over all rows.

`CI-ALT-021` MUST: weight = group priority + priority weight; sort by weight descending, then
by text ascending, then stable. Acceptance: property test: any permutation of the input yields the
same output (up to equal text and equal weight ties, which keep input order).

`CI-ALT-022` MUST: separators between groups and for rows that ask for them. Acceptance:
screenshot of S-ALT-1 shows separators between the error block, the inspection block and the
intention block.

`CI-ALT-023` SHOULD: a per-language order-provider extension replacing 5.2. Acceptance: a test
language reverses the order and the popup follows.

`CI-ALT-024` MUST: the first row is selected when the popup opens, with the whole list
scrollable and no row skipped. Acceptance: pressing Enter right after Alt+Enter applies the
first row.

## 6. Submenus (the right-arrow lists)

A row's submenu is a nested popup with title = the row's *tool name* (the inspection display
name; for plain intentions the family name) built by the same assembly and ordering rules
(5.1 and 5.2) over the option actions, with the options classified as plain intentions (group
`OTHER`, weight 0 + priority) unless they are themselves error or inspection fixes.

### 6.1 Options of an inspection or annotator fix row

In generation order (the visible order follows from 5.2: HIGH options first, then alphabetical):

| Option (text) | Priority | Available when | Effect |
|---|---|---|---|
| `Edit inspection profile setting` | HIGH | always | opens Settings at the inspection's page with the tool selected |
| `Fix all '<inspection display name>' problems in file` | HIGH | the fix is usable in batch (it starts in a write action, or is a pure/batch fix), the tool is not "unfair", and there is an editor | re-runs this tool on the **whole file** in batch mode and applies **this kind of fix** to every problem it reports |
| `Run inspection on…` | HIGH | always | opens the scope dialog (see `03-inspections.md` 9.1) and runs this single inspection |
| `Disable highlighting, keep fix` | NORMAL | the tool's current level is above INFORMATION | sets the tool's level to "do not show" in the project profile; its fix stays available as an intention (CI-HLI-101) |
| `Disable inspection` | NORMAL | the tool's default level is switchable | turns the tool off in the project profile; the highlights vanish |
| suppress rows (see below) | by fix priority | the language's suppressor supplies them and the element exists | add a suppression marker |
| standard options of the tool's problem group, if any | | | |

(The first three are alphabetically `Edit inspection profile setting`, `Fix all …`,
`Run inspection on…`; so the visible order of the HIGH block is: Edit, Fix all, Run.)

Suppress rows order (Java/Kotlin standard set), by fix priority ascending: `Suppress for
statement` (10), `Suppress for statement with comment` (20), `Suppress for parameter` (30),
`Suppress for method` / `field` / `member` (40), `Suppress for class` (50), `Suppress all
inspections for class` (60). All of them are NORMAL, so they appear after the HIGH block and
among the other NORMAL options; between suppress rows the fix-priority order applies; between a
suppress row and a non-suppress NORMAL row the alphabetical rule of 5.2 applies.

### 6.2 Options of an annotator (non-inspection) fix

If the info's tool is the generic annotator pseudo-tool (no inspection behind it) there is no
`Edit inspection profile setting`, no `Disable inspection`: the submenu contains only the fix-all
option (6.3) and the *problem group's* suppress actions, if the group is suppressible.

### 6.3 Fix-all option of an error fix or pure action

For a pure action whose presentation has a fix-all option, or for an error-fix action declared
"fix-all capable", the row has the submenu entry `Apply all '<family>' fixes in file`.
Behaviour: collect every HighlightInfo currently in the file's markup that has a fix of the same
family; remember a position for each; show a progress titled with the family; apply the fixes in
**reverse text order** (so earlier offsets are not disturbed), each at its own range start,
committing the document between fixes; the whole run is one undoable command named by the
family; a fix that is no longer available when its turn comes is skipped.

### 6.4 The empty action

When an info has a tool key but none of its fixes is available at the caret (or it has no fixes
at all and no enforced attributes), a row is created whose text is the **tool display name**,
with LOW priority, no action of its own: pressing Enter on it opens its submenu; selecting it
with the Right arrow does the same (section 10.3). It exists so that `Edit inspection profile
setting`, `Disable inspection` and the suppress rows are reachable from any problem.

### 6.5 Options of a plain intention row

`Disable '<family>'`, `Edit intention settings`, and shortcut rows (CI-FIX-012/013), plus any
options the action declares itself. For an action declaring "only my options" the standard
rows are not added.

`CI-ALT-030` MUST: the submenu rows `Edit inspection profile setting`, `Disable inspection`
and the language's suppress rows for every inspection fix row and for the empty action; their
texts exactly as in the table. Acceptance: S-ALT-1 submenu.

`CI-ALT-031` MUST: the submenu is ordered by the same algorithm (5.2) and the HIGH rows come
first. Acceptance: for an inspection fix, the first submenu row is `Edit inspection profile
setting`.

`CI-ALT-032` SHOULD: `Fix all '<name>' problems in file` as in 6.1, applying only the fix kind
of the row, in one command, to all problems of the tool in the file. Acceptance: file with 5
unused imports; Alt+Enter on one; submenu `Fix all 'Unused import' problems in file`; after it
no unused import remains and one Undo restores all five.

`CI-ALT-033` SHOULD: `Apply all '<family>' fixes in file` as in 6.3. Acceptance: file with
three identical "Add missing import" errors fixable by the same family; one invocation fixes
all; undo restores all.

`CI-ALT-034` MUST: `Disable inspection` and `Disable highlighting, keep fix` behave as in 6.1
and trigger a daemon restart for all files. Acceptance: after choosing, the project profile
file records the change and all editors re-highlight.

`CI-ALT-035` MUST: the empty action of 6.4 exists exactly when specified: once per
(range, tool) pair per run and never for the annotator pseudo-tool. Acceptance: an inspection
without fixes shows a one-row popup (the tool name) with a submenu arrow; Enter opens it.

`CI-ALT-036` SHOULD: the suppress rows' order by fix priority. Acceptance: the Java submenu
order of the scenario S-ALT-1.

## 7. `ModCommand`: the data model of "what an action does"

A `ModCommand` is a *value* (a record of fields, no behaviour) describing effects. It can be
serialised, previewed, replayed in a different process, and executed in different modes.

### 7.1 Variants and meanings

| Variant | Fields | Meaning |
|---|---|---|
| **Nothing** | | no effect |
| **Composite** | ordered list of commands | executed in order; empty/nothing elements dropped; nested composites are flattened |
| **UpdateFileText** | file, old text, new text, list of *updated ranges* (old range, new range, with flag whether the range grew) | replace the text of an existing file; the executor re-checks that the file still has the old text (optimistic check); the ranges let the executor update the caret/selection mapping |
| **CreateFile** | target file (a future file: directory + name), content (text or binary) | creates a file |
| **DeleteFile** | file | deletes a file |
| **MoveFile** | file, target directory | moves a file |
| **UpdateReferences** | file, old range, new range | refreshes references in a range (used after moves) |
| **Navigate** | file, caret offset, optional selection range | moves the caret and selection in the (possibly other) file's editor; opens it if needed |
| **Highlight** | file, list of (range, attributes key) | temporary highlight, removed on the next caret move or edit |
| **StartRename** | file, list of (rename range, optional suggested names), "name suggestions" | starts the in-place rename session on the given ranges |
| **StartTemplate** | file, template builder state: ordered list of *variables* (name, expression or fixed value, default text, "always stop at"), the template text with variable positions, final caret position | starts a live-template-like session |
| **RegisterTabOut** | file, range, target offset | Tab at the range's end jumps to the offset (so a user can tab out of a generated bracket) |
| **ChooseAction** | popup title, list of `ModCommandAction`s | asks the user to choose one; then executes the chosen action's command |
| **ChooseMembers** | title, list of members with default selection, mandatory ones, single/multi selection, a function from the chosen list to a command | member selection dialog |
| **ShowConflicts** | map element → conflict messages | shows a conflict dialog; the user can continue or cancel |
| **EditOptions** | option page identifier, option changes (new values) | updates inspection or system options (maybe asks confirmation) |
| **UpdateSystemOptions** | option controller changes | changes settings |
| **DisplayMessage** | message, kind (information, error) | shows a hint in the editor |
| **OpenUrl** | url | opens the browser |
| **CopyToClipboard** | text | puts text on the clipboard |
| **LaunchEditorAction** | action id, optional arguments | runs an editor action (for example "start in-place refactoring") |

Helpers every implementation provides: `nop`, `error(message)`, `info(message)`,
`select(element)`, `moveTo(element)`, `highlight(...)`, `andThen(next)`, `unpack()`,
`isEmpty()`, `modifiedFiles()`, `psiUpdate(context, updater)`, `psiUpdate(element, updater)`
(the *update on a copy* idiom of 7.2), `chooseAction`, `psiUpdateStep(element, title, updater)`.

### 7.2 The "update on a copy" idiom

Most fixes edit syntax. Instead of mutating the live tree, `perform` calls
`psiUpdate(element, updater)`: the engine takes a **non-physical copy** of the file, hands the
updater the *copy of that element* (and a small updater object to set caret, selection, choose
members, start rename or templates on the copy), the updater mutates the copy with ordinary
tree operations, and the engine diffs the copy's text with the original to produce an
`UpdateFileText` plus navigation. Effects: the same code runs for the preview and for the
real edit; nothing needs a write lock in the background; races with user typing are caught by
the optimistic text check.

### 7.3 Executors (modes)

| Mode | Used for | Behaviour |
|---|---|---|
| **Interactive** | the user picked the row | runs on the UI thread inside one *command*; opens one undo group across *all* touched files; asks questions (choose, conflicts) through real UI; applies file edits, then caret/selection/templates; selects the final caret position |
| **Batch** | fix-all, cleanup, Inspect Code | no UI questions; `ChooseAction` takes the first action, conflicts are accepted; applies edits per file; navigation and templates are ignored |
| **Preview** | the preview popup | applies the file-edit variants to copies in memory, never touches disk or settings, ignores navigation/UI variants (renders "navigate", "rename", "create file" as text), returns an `IntentionPreviewInfo` |
| **For a file copy** | previews and nested updates | applies only `UpdateFileText` against the supplied copy |

Rules common to all executors:
1. Before applying, check the files are writable; if not, prompt once ("make writable") and
   abort on refusal.
2. `UpdateFileText` fails the whole command if the file text no longer matches `old text`
   (the action's data is stale); the user sees nothing changed.
3. Composite commands are all-or-nothing as far as undo is concerned: one undo step.
4. After applying text, the caret is placed by the last `Navigate` (or the mapped original
   caret).
5. Ranges listed in `UpdateFileText` are used to keep the editor's caret and selection stable
   across the edit.
6. Document commit to the syntax tree happens between the dependent steps.

`CI-FIX-020` MUST: the variants Nothing, Composite, UpdateFileText, CreateFile, DeleteFile,
MoveFile, Navigate, Highlight, StartRename, ChooseAction, ShowConflicts, DisplayMessage, OpenUrl,
CopyToClipboard (for the Java/Kotlin/Spring MVP: all fixes need at least these). Acceptance:
each variant has a unit test of its interactive and preview execution.

`CI-FIX-021` MUST: the interactive executor of 7.3 (one undo group over all touched files, one
command, optimistic check, writable check). Acceptance: a command editing two files is undone by one Undo step that reverts both files
(project-wide undo group).

`CI-FIX-022` MUST: the preview executor of 7.3. Acceptance: previewing an action that creates a
file shows the new file name and content, and no file is created.

`CI-FIX-023` MUST: the batch executor of 7.3. Acceptance: fix-all over 10 problems never opens
a popup.

`CI-FIX-024` SHOULD: StartTemplate, RegisterTabOut, ChooseMembers, EditOptions,
UpdateSystemOptions, UpdateReferences, LaunchEditorAction. Acceptance: "Create method" fix
starts a template with the parameter list and the Tab-out registration.

`CI-FIX-025` MUST: the "update on a copy" idiom: `psiUpdate` yields an `UpdateFileText` whose
diff is minimal (only changed ranges) and has the mapped caret. Acceptance: an action
that changes one token yields an UpdateFileText with one range pair.

`CI-FIX-026` MUST: the Archipel proposal in `docs/analysis/code-intelligence-spec.md` 1.9
(`ModCommand` enum with UpdateFile{expected_rev}, Navigate, StartRename, StartTemplate,
ShowConflicts, Choose, ...) is *compatible*: it needs additionally `Highlight`, `ChooseMembers`,
`EditOptions/UpdateSystemOptions`, `MoveFile/UpdateReferences` and the `updated ranges` list.
Acceptance: the enum covers the table.

## 8. Intention preview

A **preview** shows what choosing the selected row would change, next to the popup, before the
user commits.

Behaviour:
- Setting *Show preview for intention actions when available* (Settings | Editor | General |
  Appearance, default on). The popup advertises `Press <shortcut> to toggle preview` where
  `<shortcut>` is the active keymap's *Quick Documentation* shortcut (F1 on macOS default keymap,
  Ctrl+Q elsewhere). Pressing it toggles the preview for the session of this popup.
- The preview appears to the side of the popup (to the right when there is room, otherwise to
  the left), updates when the selected row changes (moving the selection or hovering), shows
  `Loading…` while computing (computed in a background task, cancelled when the selection
  changes), and is destroyed with the popup.
- Content kinds (`IntentionPreviewInfo`): **diff** (before/after of the current file with
  whitespace-trimmed comparison and changed hunks with small context), **diff without trimming**,
  **diff in another file** (file name header), **multi-file diff**, **custom diff** (two texts
  with file type), **HTML** (description + before/after), **empty**, **fallback diff** (apply the
  action to a copy and diff).
- Computation: `generate preview` of the action runs against a **non-physical copy** of the
  file with an in-memory editor positioned at the fix offset; any attempt at a side effect (file
  system, settings, clipboard, UI) raises a "side effect not allowed" error which is logged and
  yields no preview. For a pure action the command is previewed with the preview executor.
- If the diff cannot be produced: use the intention's registered description (before/after
  samples) as HTML; else show `No preview available`. If some rows have no preview the popup's
  ad text reads `Some actions have no preview`.
- For multi-fix rows whose submenu is open, the preview follows the submenu selection.
- Ranges listed by the presentation (affected, deleted) are highlighted in the editor while the
  row is selected (search-result colour for affected, strikethrough for deleted).

`CI-ALT-040` SHOULD: the preview of section 8 for pure actions (diff kinds, `Loading…`,
`No preview available`, toggle shortcut ad, setting default on). Acceptance: S-ALT-1: selecting
the import fix shows a diff adding the import line.

`CI-ALT-041` SHOULD: the editor highlighting of presentation ranges while selected.
Acceptance: selecting `Remove unused import` strikes through the import text.

`CI-ALT-042` LATER: HTML/custom/multi-file previews.

## 9. The light bulb in the editor

### 9.1 When it is shown

The *show intentions* pass (`CI-DMN-004`) recomputes the five sets for the caret; if the result
changed, the bulb is updated on the UI thread **only if all** hold:
- the editor has keyboard focus (not in test mode);
- the caret's line is inside the visible area (the caret's pixel position is visible);
- the editor is not a viewer, the setting *Show intention bulb* is on (Settings | Editor |
  General | Appearance, default on), and exactly one caret exists;
- at least one row has `show light bulb` true and its family is not on the user's "ignored"
  list;
- no other hint that would be hidden by a new hint is showing;
- the user did not just press Esc (a flag cleared by the next caret move).
Otherwise the existing bulb is hidden.

### 9.2 Which bulb

Evaluated in this order:
1. old UI only: if an inspection-fix row is a refactoring-type intention, a "refactoring" bulb;
2. if any error-fix row has `show light bulb`: the **red** bulb (quick-fix bulb);
3. else if any inspection-fix row has a bulb: if exactly one distinct custom bulb icon is
   declared among those rows (and the action allows overriding the icon), use it; else the
   **yellow** intention bulb;
4. else (only plain intentions): if exactly one distinct custom icon is declared among all rows,
   use it; else the **yellow** bulb.

In the popup the row icons follow: error-fix rows show the red-bulb icon, inspection-fix rows
the yellow-bulb icon, plain intention rows none (new UI) and rows of actions that opt out of
the bulb show the "off" variants; rows with their own icon show it.

### 9.3 Position and interaction

- Anchored at the **left edge of the visible text area** (not at the caret column). If the
  caret line's text starts to the right of the bulb's width plus margin, the bulb is vertically
  centred on the caret line; otherwise it is placed on the line **above** the caret (above any
  block inlay of that line), and if there is no room above (the caret is on the first visible
  line), on the line **below**. It is invisible when no room exists.
- The icon size is 16 px with a transparent border of 6 px (4 px in single-line editors) that
  becomes a visible 1 px frame in the selection tear-line colour on hover.
- Click (primary button) opens the popup below the bulb. The popup is the same as Alt+Enter.
- The bulb hides on typing, scrolling away, caret leaving the line, a modal change, or a new
  bulb replacing it. It is delayed by a short interval when the caret moves so it does not
  flicker while arrow-key navigating.

`CI-ALT-050` MUST: bulb show conditions 9.1 and bulb choice 9.2 (red when any error fix is
shown, yellow otherwise). Acceptance: caret on an unresolved symbol with an import fix: red bulb;
caret in a statement with only an intention: yellow bulb; caret on a warning with a fix:
yellow; caret on a clean token with no actions: no bulb.

`CI-ALT-051` MUST: bulb position rule 9.3. Acceptance: a line whose text starts at column 0 gets
the bulb on the line above; a line indented by 8 columns gets it in the line.

`CI-ALT-052` MUST: the settings *Show intention bulb* (default on) and the per-intention
*ignored* list. Acceptance: disabling the setting hides the bulb but Alt+Enter still works.

`CI-ALT-053` MUST: clicking the bulb opens the popup and hovering shows the frame. Acceptance
visual.

## 10. Invocation and keyboard behaviour

### 10.1 Alt+Enter (action *Show Intention Actions*, `Alt+Enter`)

Steps of the action handler, in order:
1. commit pending document changes to the syntax tree;
2. if a **completion popup** is showing: show the *element actions* of the selected completion
   item instead (LATER) and stop;
3. let **auto-import** complete: run the language's reference importers for the reference at the
   caret: if "Add unambiguous imports on the fly" is on for the file and the reference at the
   caret has exactly one import candidate, import it right now (undo-transparent) — the popup
   then opens for the new state (see `05-auto-import.md` CI-IMP-040);
4. hide the existing bulb;
5. if an **import hint question** is currently shown, perform it (import the class, or open the
   class chooser) **instead** of showing the popup, and stop;
6. if a **template session** is active, finish it (move to its end) in its own command;
7. scroll the caret into view (minimal scroll);
8. compute the five sets for the caret (the same assembly as 4, with a progress "Searching for
   context actions…" that appears only if it takes long) and the final list. If it is **empty**,
   nothing is shown (when invoked from the editor context menu *Show Context Actions*, an
   information hint `No context actions available at this location` is shown instead);
9. after scrolling finished, show the popup at the best position near the caret (below the
   caret line, flipped above if no room); the first row is selected; the preview opens if
   enabled.

### 10.2 The popup widget

- A list popup, no title in the new UI for the root list (submenus have their tool name as
  title), rows with icon column (aligned across rows; rows without icon get an empty icon of
  the maximal icon width), text, optional shortcut text at the right, submenu arrow at the
  right for rows with a submenu.
- **No mnemonic navigation** (digits and letters do not jump). **Speed search is on**: typing
  letters filters rows by substring (camel humps allowed) of the row text with highlighted
  matches; Backspace edits; Esc clears the filter, then closes.
- Keys: `Up`/`Down` move the selection (wrapping at the ends), `PageUp/PageDown`, `Home/End`
  first/last; `Enter` activates; `Esc` closes (and sets the *escape just pressed* flag that
  suppresses the bulb until the next caret move); `Right` opens the submenu of the selected row
  (if it has one), `Left` closes the submenu and returns; these are the standard list-popup
  keys of the toolkit and are not specific to intentions.
- Mouse: hover selects; click activates; clicking the submenu arrow opens the submenu.
- **Enter on a row with a submenu**: normal rows are *final* choices: Enter applies the row's
  action (the submenu is opened with `Right` only). The **empty action** row is not final when it
  has a submenu: Enter opens the submenu.
- The popup closes when the editor loses focus, the caret moves, or the document changes.
- Closing the popup by choosing a row hides all hints first.

### 10.3 Applying a row

1. Run in a *command* named with the **row text** (the Undo menu shows `Undo <row text>`).
2. If the action is not usable in the current dumb state: show the notification `'<text>' is
   not available during indexing` (in "light mode": `... is not available in light mode`) and do
   nothing.
3. Commit documents; re-resolve the file for the editor (host or injected, the one where the
   action is available at the caret).
4. If the row's fix offset is known and the caret is not there (a **remote error** fix): move
   the caret to the fix offset for the duration of the invocation; afterwards restore the
   original caret offset if the caret is still at the fix offset and no template session started.
5. Make the element/file writable (prompt if read-only); abort on refusal.
6. Invoke (in a write action when the action says so; pure actions via the executor).
7. After a successful invoke of a *quick fix*, unregister that fix from the infos near the caret
   (so it is not offered twice before the next analysis) and remove the row from the cache.
8. The daemon restarts as for any edit.

`CI-ALT-060` MUST: steps 1 to 9 of 10.1, in particular the auto-import pre-step (3), the
pending-import-hint step (5) and the empty-menu behaviour. Acceptance:
S-ALT-2 and S-ALT-4.

`CI-ALT-061` MUST: popup widget behaviour of 10.2 (no mnemonics, speed search, key bindings,
Enter vs Right on rows with submenus, empty action behaviour). Acceptance: keyboard test script:
Down, Down, Right opens submenu, Esc closes the submenu only, Esc closes the popup.

`CI-ALT-062` MUST: the apply sequence 10.3 including the command name, dumb-mode
notification, remote-error caret handling and unregistering. Acceptance: after applying
`Import class`, Undo shows `Undo Import class` and restores the file.

`CI-ALT-063` SHOULD: speed search in the popup with substring/camel-hump matching.
Acceptance: typing `susp` filters to rows containing "Suppress".

## 11. LSP code actions in the same popup (Archipel)

Language-server `textDocument/codeAction` results are a *fourth producer*:

| LSP item | Becomes | Group |
|---|---|---|
| `quickfix` kind with `diagnostics` that match a HighlightInfo at the caret (same range, `diagnostic.severity` error) | error fix attached to that info (tool id `lsp:<server>`) | `ERROR` / `REMOTE_ERROR` by the same fix-range rule |
| `quickfix` kind for a warning/hint diagnostic | inspection fix | `INSPECTION` |
| `quickfix` kind with no matching info | intention | `OTHER` |
| `refactor.*`, `source.*`, `refactor.rewrite` kinds | intention | `OTHER` |
| item with `isPreferred = true` | priority HIGH | by kind |
| item with `disabled` | not shown | |
| `command` only item | wrapped: executes the command through the server | by kind |

De-duplication against native rows: compare (family-or-kind, text); the native row is kept and
the LSP row dropped. The submenu of an LSP quick fix row has `Edit inspection profile setting`
replaced by `Configure language server` (LATER).

`CI-ALT-070` SHOULD: the mapping above, with preference for native rows. Acceptance: jdtls
returns `Import 'Foo' (java.util)` for an unresolved `Foo` that the native importer also
offers: one row, the native one.

## 12. UX acceptance scenarios

**S-ALT-1, unresolved static method (Java, Gradle, AssertJ on the classpath).**
Input: caret at the end of `assertThat` in `assertThat(result).isEqualTo(5);`, no import.
Expected popup, in this order (example texts; the Java spec owns exact fix texts):
1. `Import static method 'org.assertj.core.api.Assertions.assertThat'` (error group, TOP,
   weight 50; with several candidates the text is `Import static method...`);
2. further error fixes of normal priority, alphabetical, e.g. `Create method 'assertThat'`,
   `Rename reference` (weight 30);
3. separator; if the info has an inspection tool behind it, the tool-name row;
4. separator; applicable plain intentions (weight 0).
Right arrow on row 1: the fix comes from the general pass (the annotator pseudo-tool, no
inspection), so the submenu contains only the fix-all entry (6.2 and 6.3) and no
`Edit inspection profile setting` row.
Enter on row 1: if exactly one candidate, the file gets `import static org.assertj.core.api.Assertions.assertThat;`
in the import list at the position given by the import layout, the caret returns to where it
was, the red underline disappears after the next daemon cycle; Undo restores the file.
Preview (toggle on): a one-hunk diff adding the import line.

**S-ALT-2, ambiguous import via Alt+Enter without the on-the-fly setting.** Caret on `List`
(unresolved) with candidates `java.util.List`, `java.awt.List`. Popup row 1: `Import class`.
Enter opens the *Class to Import* chooser (title `Class to Import`, at most 25 entries, ordered
by the Java import-candidate ranking, first row selected); Enter on `java.util.List` imports.

**S-ALT-3, remote error.** Line `Foo f = make(); int x = bar(1);` where `bar` is unresolved; caret
on `Foo` (resolved). Popup contains `bar`'s error fixes in group REMOTE_ERROR (weight 10 + priority)
above plain intentions (weight 0..3) and below any inspection fixes (20). Applying one of them
moves the caret to `bar` during the fix and restores it to `Foo` afterwards.

**S-ALT-4, pending import hint.** The editor shows the hint `Import class 'java.util.List'? Alt+Enter`
(see `05-auto-import.md`) when the user presses Alt+Enter: the class is imported (or the chooser
opens); no popup appears.

**S-ALT-5, hidden-problem fix.** Inspection "Unused declaration" set to "No highlighting, only
fix": caret on an unused private field: no squiggle; Alt+Enter lists the fix (for example `Safe
delete 'field'`) among plain intentions, and its submenu offers `Edit inspection profile setting`.

**S-ALT-6, fix all in file.** Five unused imports. Caret on one. Alt+Enter: row `Optimize imports`
(inspection group) and the tool-name row `Unused import` whose submenu starts with `Edit
inspection profile setting`, `Fix all 'Unused import' problems in file`, `Run inspection on…`
then `Disable highlighting, keep fix`, `Disable inspection`, `Suppress for statement`... Choose
`Fix all ...`: all five imports vanish; Undo brings them back in one step.

**S-ALT-7, empty popup.** Caret in a blank line of a comment: no bulb; Alt+Enter shows nothing;
from the context menu entry *Show Context Actions* the hint `No context actions available at
this location` appears.

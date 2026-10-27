# 02. The `HighlightInfo` model

Status: specification, language independent. Requirement prefix `CI-HLI`.
Priorities: `MUST` / `SHOULD` / `LATER` as in `01-daemon.md`.

`HighlightInfo` is the single record type that every analysis producer emits and every consumer
reads: the editor markup, the error stripe, the status widget, the Problems view, F2 navigation,
the Alt+Enter popup, the import hints and the tooltips. If two features show different problems
for the same file, the cause is that one of them does not read `HighlightInfo`.

ic: `platform/analysis-impl/src/com/intellij/codeInsight/daemon/impl/HighlightInfo.java`,
`HighlightInfoType.java`, `UpdateHighlightersUtil.java`, `SeverityRegistrar.java`;
`platform/analysis-api/src/com/intellij/lang/annotation/HighlightSeverity.java`;
`platform/core-api/src/com/intellij/openapi/editor/colors/CodeInsightColors.java`.

## 1. Fields (conceptual data model)

| Field | Meaning | Rules |
|---|---|---|
| range (start, end) | The text the problem is about. | `0 <= start <= end <= text length`. Empty ranges are allowed; they are drawn at the end of the previous line when the "after end of line" flag is set, and otherwise are dropped by display filters unless they carry a gutter icon or fixes. |
| type (`HighlightInfoType`) | A named pair of *severity* and *text attributes key*. | Equality is by severity and key. See section 4. |
| severity | One value of the severity scale (section 3). | Comes from the type, except for types whose severity is looked up by the user's inspection profile (unused symbol, deprecation). |
| description | Plain text message, shown in the status line / Problems view / screen reader. | Never HTML. May be null for purely visual infos. |
| tooltip | HTML shown on hover and in the error stripe popup. | Defaults to the escaped description when not given. Inspection tooltips append a "more…" link (section 7). |
| forced attributes / forced key | Overrides the type's attributes. | A concrete attribute set wins over a key, a key wins over the type's key. |
| after end of line | Draw the (empty-range) marker after the last character of the line. | |
| file level | The problem concerns the whole file, not a text range. | Shown in a banner above the editor text instead of a range highlight (section 9). |
| gutter icon | Optional icon with click action and tooltip. | Used by annotators that want a gutter mark. |
| problem group | Optional grouping object: its *problem name* is the inspection key used for suppress/disable submenus, and it can offer its own suppress actions. | |
| tool id | Who produced this info: an inspection's short name, an annotator or visitor identity, an external tool id. | Used for per-tool replacement (CI-DMN-050), "disable inspection" fixes and the fix-all submenu. |
| group | Small integer naming the *pass* that produced the info. | Used to compute fixes only for the pass being queried (for example the intentions pass of one group). |
| quick fixes | List of fix descriptors, each with: the action, an optional list of submenu options, a display name for the producing tool, an icon, the tool key, the problem group, the severity of the problem for which it was registered, and a **fix range**. | Section 6. |
| navigation shift | Offset added to `start` when F2 jumps to the info. | Default 0. |
| from injection | The info was computed inside an injected language fragment and translated to host offsets. | |

`CI-HLI-001` MUST: the record carries all fields above. Acceptance: a unit test builds an info
with every field and reads each one back; building with `start > end` or `start < 0` is
rejected with a clear error message naming the description.

`CI-HLI-002` MUST: ranges are stored as range markers while attached to a document (the info
moves with edits) and fix ranges are separate markers (section 6). Acceptance: insert text
before an info; its highlighter and fix range move by the same amount.

`CI-HLI-003` MUST: producers use a builder to create infos. The builder returns *nothing*
(no info) when a global filter rejects the candidate (section 5). Acceptance: building an
INFORMATION info with no attributes, no tooltip, no gutter icon and no fixes yields nothing.

## 2. Descriptions and tooltips: exact text rules

1. A description is a single line of plain text without trailing period conventions imposed by
   the platform, but when several descriptions are joined (composite infos, section 7) each one
   is trimmed, gets a period appended if it does not end with one, and is followed by a space.
2. Inspection descriptions come from the `ProblemDescriptor` template (see
   `03-inspections.md` section 4); a descriptor may also carry a separate *tooltip template*
   (default: same as the description template). Rendering steps, in order:
   a. In a template that is not wrapped in `<html>`, `<code>` and `</code>` become single
      quotes.
   b. `#loc` (a location placeholder) is removed together with one adjacent space.
   c. XML entities in non-HTML templates are decoded.
   d. `#ref` is replaced by the text of the highlighted range (or of the element), with runs
      of whitespace collapsed to one space and truncated to 100 characters (ellipsis at the end
      when truncated).
   e. The markers `#end` and `#treeend` are deleted. (`#treeend` additionally cuts the text off
      for *tree* presentations such as the Inspect Code results view, not for the editor.)
   f. The result is trimmed.
   The editor *description* is additionally reduced to plain text when it starts with
   `<html>`: tags removed, entities decoded, `&nbsp;` and `&#32;` turned into spaces.
3. A tooltip is HTML wrapped in `<html>...</html>`. When a tooltip was not set explicitly it is
   the description with XML characters escaped.
4. The unresolved-reference default message is `Cannot resolve symbol '<name>'` where `<name>`
   is the reference's canonical text. A reference class may supply its own pattern.

`CI-HLI-010` MUST: description is plain text; tooltip is HTML; default tooltip is the escaped
description. Acceptance: description `a < b` yields tooltip `<html>a &lt; b</html>`.

`CI-HLI-011` MUST: the `#ref`, `#loc`, `#end`, `#treeend`, `<code>` template processing of
rule 2. Acceptance: template `Field <code>#ref</code> is never used #loc` on the identifier
`count` gives the editor message `Field 'count' is never used`; a 150-character `#ref` is cut to
100 characters with an ellipsis.

`CI-HLI-012` MUST: the default unresolved message of rule 4. Acceptance: an unresolved name
`foo` yields `Cannot resolve symbol 'foo'` unless the language overrides it (Java overrides:
`Cannot resolve method 'assertThat'` and similar, see the Java spec).

## 3. The severity scale

Severities are compared by an integer. Values and user-visible names:

| Name (internal) | Value | Capitalised UI name | Count message (singular / plural) | Where it comes from |
|---|---|---|---|---|
| INFORMATION | 10 | Information | `{n} information` | Pure information: no squiggle; often only a tooltip, a gutter or a fix. Also the "do not show" profile level (the problem is hidden but its fix remains available). |
| TEXT ATTRIBUTES | 11 | Consideration | `{n} consideration` | A highlight that only changes text attributes (for example "Edit as text style"); never counted as a problem in navigation. |
| SERVER PROBLEM | 100 | Server Problem | `{n} server problem(s)` | External server results with no more specific severity. |
| INFO (deprecated) | 200 | Info | `{n} info(s)` | Legacy; same value as WEAK WARNING. Treat as WEAK WARNING in new code. |
| WEAK WARNING | 200 | Weak Warning | `{n} weak warning(s)` | Suggestions that are probably right but not worth a warning. |
| WARNING | 300 | Warning | `{n} warning(s)` | |
| ERROR | 400 | Error | `{n} error(s)` | Code that will not compile or run correctly. |

Plural forms: the count message uses the usual 0/1/other split: `0 errors`, `1 error`,
`2 errors`. A custom severity's count message is `{count} {name}`.

Additional **internal** severities exist and must never appear in inspection profiles or in the
status counts:

| Internal severity | Value | Use |
|---|---|---|
| injected fragment syntax | 6 | syntax colouring inside injected fragments |
| injected fragment | 7 | background of injected fragments |
| symbol type | 8 | "semantic highlighting" attributes (field vs local, etc.) and "possible problem" markers (no colour, used so redundant-suppression knows a place was not fully checked) |
| highlighted reference | 7 | reference-under-cursor style highlighting |
| element under caret | 401 (above ERROR) | identifier highlighting; paints over everything and is excluded from counts |

**The severity registry** orders all severities for counts and for the severity combo box in the
inspection settings. The default order is the numeric order of the table. The user can add
*custom severities* in Settings | Editor | Inspections | Severities (a name, text attributes,
and a position relative to the standard ones); plugins can register *standard* severities
(for example "Typo"). Changing the registry restarts the daemon for all files.

`CI-HLI-020` MUST: the seven public severities with the values above and the compare
operation by value. Acceptance: `ERROR > WARNING > WEAK WARNING = INFO > SERVER PROBLEM >
TEXT ATTRIBUTES > INFORMATION`.

`CI-HLI-021` MUST: a severity registry with a total order (indexes 0 to n), used by counts
(CI-DMN-110), by F2 (CI-DMN-120) and by profile severity combos. Index of the standard set:
0 information, 1 text attributes, 2 server problem, 3 info, 4 weak warning, 5 warning, 6 error.
Acceptance: adding a custom severity "Blocker" between warning and error gives it index 6 and
moves error to 7; counts and F2 honour it.

`CI-HLI-022` MUST: the count messages with the singular/plural rules above. Acceptance: the
status widget tooltip for one weak warning reads `1 weak warning`, for none of a severity the
chip is not shown.

`CI-HLI-023` SHOULD: custom and plugin-registered severities (name, attributes, order) with
persistence in the application settings. Acceptance: define "Blocker" with a purple wave; set an
inspection to it; the editor shows a purple wave.

`CI-HLI-024` MUST: internal severities are not selectable in profiles and not counted.
Acceptance: the severity combo lists exactly the registry's public severities that are
applicable to inspections.

## 4. `HighlightInfoType` and text attribute keys

A `HighlightInfoType` maps to *one text attributes key*; the key names are the stable
identifiers stored in colour schemes, so use these exact strings.

| Type | Severity | Text attributes key (scheme key name) | Typical use |
|---|---|---|---|
| ERROR | ERROR | `ERRORS_ATTRIBUTES` | general errors (type mismatch, syntax) |
| WRONG_REF | ERROR | `WRONG_REFERENCES_ATTRIBUTES` | unresolved references (red *text*, not a squiggle, in the default scheme) |
| UNHANDLED_EXCEPTION | ERROR | `ERRORS_ATTRIBUTES` | unreported exception |
| WARNING | WARNING | `WARNING_ATTRIBUTES` | warnings |
| WEAK_WARNING | WEAK WARNING | `INFO_ATTRIBUTES` (the weak-warning key's external name is the same string as the legacy info key) | weak warnings |
| INFO (deprecated) | INFO | `INFO_ATTRIBUTES` | legacy |
| INFORMATION | INFORMATION | `INFORMATION_ATTRIBUTES` | tooltip/fix-only infos |
| TEXT_ATTRIBUTES | TEXT ATTRIBUTES | `CONSIDERATION_ATTRIBUTES` | style-only highlight |
| GENERIC_WARNINGS_OR_ERRORS_FROM_SERVER | SERVER PROBLEM | `GENERIC_SERVER_ERROR_OR_WARNING` | server results |
| UNUSED_SYMBOL | (profile: tool "unused") | `NOT_USED_ELEMENT_ATTRIBUTES` | grey-out of unused things |
| DEPRECATED | (profile: tool "deprecation") | `DEPRECATED_ATTRIBUTES` | strike-through |
| MARKED_FOR_REMOVAL | (profile: tool "forRemoval") | `MARKED_FOR_REMOVAL_ATTRIBUTES` | strike-through in a stronger colour |
| TODO | INFORMATION | `TODO_DEFAULT_ATTRIBUTES` | TODO comments |
| POSSIBLE_PROBLEM | symbol type (internal) | no highlighting | "not fully checked" marker |
| ELEMENT_UNDER_CARET_READ / _WRITE / _STRUCTURAL | element under caret (internal) | `IDENTIFIER_UNDER_CARET_ATTRIBUTES` / `WRITE_IDENTIFIER_UNDER_CARET_ATTRIBUTES` / `MATCHED_BRACE_ATTRIBUTES` | identifier highlighting |

Types whose severity is "by profile": the severity of an unused-symbol, deprecation or
for-removal info is the *current level of that inspection in the active profile for that file*
(so the user can turn deprecation into an error, or hide unused-symbol greying by setting its
level to "do not show").

### 4.1 Attribute resolution order (what the editor paints)

1. If the info has concrete forced attributes, use them.
2. Else if it has a forced key, resolve it in the editor's colour scheme.
3. Else if it comes from an inspection with `GENERIC_ERROR_OR_WARNING` highlight type and the
   inspection profile entry for this tool names an *editor attributes key* (an inspection can
   say "paint me with key X" in its registration or profile), resolve that key.
4. Else resolve the type's key. If the scheme has no attributes for a *severity-named* key,
   fall back to the severity's own attributes from the registry (custom severities).
5. An info whose resolved attributes are empty and whose severity is INFORMATION is dropped
   unless it has fixes, a gutter icon or a tooltip (see section 5).

Default appearance in the bundled schemes (facts from the colour scheme data, `docs/spec/data/
color-schemes.json`; effect type numbers: 0 box, 1 underline, 2 wave, 3 strike-through,
4 bold underline, 5 bold dotted line):

| Key | Default (light) | Darcula | Islands Dark |
|---|---|---|---|
| `ERRORS_ATTRIBUTES` | wave `ff0000`, stripe `cf5b56` | wave `bc3f3c`, stripe `9e2927` | wave `fa6675`, stripe `d64d5b` |
| `WRONG_REFERENCES_ATTRIBUTES` | foreground `ff0000` | foreground `bc3f3c` | foreground `f75464` |
| `WARNING_ATTRIBUTES` | background `f6ebbc`, stripe `ebc700` | background `52503a`, stripe `be9117` | wave `f2c55c`, stripe `c29e4a` |
| `INFO_ATTRIBUTES` (weak warning) | wave `cccccc`, stripe `d9cfad` | wave `aeae80`, stripe `756d56` | wave `857042` |
| `NOT_USED_ELEMENT_ATTRIBUTES` | foreground `808080`, stripe `ebc700` | foreground `72737a`, stripe `be9117` | foreground `6f737a` |
| `DEPRECATED_ATTRIBUTES` | strike-through `404040` | strike-through `c3c3c3` | strike-through `bcbec4` |
| `MARKED_FOR_REMOVAL_ATTRIBUTES` | strike-through `ff0000` | strike-through `bc3f3c` | strike-through `f75464` |
| `GENERIC_SERVER_ERROR_OR_WARNING` | wave `f49810`, stripe `e69317` | underline `f49810`, stripe `b06100` | same as Darcula |
| `TODO_DEFAULT_ATTRIBUTES` | italic `0073bf`, stripe `54aae3` | italic `a8c023`, stripe `0977ab` | italic `8bb33d`, stripe `73ad2b` |
| `IDENTIFIER_UNDER_CARET_ATTRIBUTES` | background `e4e4ff` | background `344134` | background `373b39` |
| `WRITE_IDENTIFIER_UNDER_CARET_ATTRIBUTES` | background `ffe4ff` | background `40332b` | background `402f33` |

(The Archipel kit takes colours from the theme through `Theme::colors`; the table is the
reference to compare against, not a source of literals.)

`CI-HLI-030` MUST: type table with the exact key names above; each type has exactly one key
and one severity (or by-profile). Acceptance: a conformance test enumerates the table.

`CI-HLI-031` MUST: attribute resolution order of 4.1. Acceptance: forced attributes beat a
forced key, which beats the type key; an inspection set to level WARNING with a profile
editor-attributes key `MY_KEY` paints with `MY_KEY`.

`CI-HLI-032` MUST: by-profile severities for unused, deprecated and for-removal infos.
Acceptance: set the deprecation inspection to ERROR in the profile; a deprecated call is an
error in the counts and gets a stripe mark in the error colour, drawn with the deprecation
attributes (strike-through).

`CI-HLI-033` MUST: the bundled colour schemes provide every key of the table, so a custom
scheme that lacks a key falls back to its parent scheme's attributes. Acceptance: remove
`WARNING_ATTRIBUTES` from a child scheme; warnings still render.

## 5. Filters

1. **Info filters (extension point).** Every candidate info is passed to all registered filters;
   if any rejects it, it is not created. Built-in rule: a file that is a *compiled/decompiled
   class* only accepts infos of the "symbol type" severity (semantic colours), never problems.
   Infos that have registered fixes are never hidden by this filter, and INFORMATION infos
   with empty resolved attributes, no gutter icon and no tooltip are rejected.
2. **Post filters** run after the info is created and before it is added to the markup.
3. **Minimum severity of a session.** The highlighting session may carry a minimum severity
   (used by "Inspect code" style runs and by the *Essential* level to drop low-severity infos).
4. **Ranges outside the document** are ignored.

`CI-HLI-040` MUST: info filters and post filters exist as extension points with the built-in
rules above. Acceptance: a test filter that rejects all WARNING infos removes warnings from the
markup and the counts.

## 6. Quick fixes attached to an info, and the fix range

- A fix is registered on an info with: the action, optional *options* (the submenu entries of
  the action, section 6.2), a *display name* (the tool's name, used as submenu title), an icon,
  the tool key (for the standard submenu entries), and an optional fix range.
- **Fix range.** The range within which Alt+Enter (and the light bulb) offers this fix. By
  default it equals the info's range. A producer may widen it, for example an *import class*
  fix is available from anywhere on the line of the unresolved name. At attach time the fix
  range becomes a range marker so it follows edits.
- **Lazy fixes.** A producer may supply a function that computes fixes later in the
  background (because they need an index search). The result is attached when ready; the daemon
  starts computing the lazy fixes of infos near the caret and in the visible range first.
  Alt+Enter waits for the lazy fixes under the caret to finish (with a progress "Searching for
  context actions…"), so the popup is complete.
- Fixes are compared by the fix *action* (equality), and an action already invoked is
  unregistered from the infos around the caret so it disappears from the popup at once.
- **Empty action.** When all fixes of an info are unavailable at the caret but the info has a
  tool key, an "empty action" whose text is the inspection's display name is added so that the
  *submenu* (Edit profile, Disable, Suppress...) stays reachable (see `04-intentions-and-alt-enter.md`).
- **Availability** is evaluated lazily, at popup time, for the actions whose fix range
  contains the caret offset or ends exactly at the caret offset (CI-ALT-010).

`CI-HLI-050` MUST: fix descriptors and fix ranges as described, including widening and
range-marker behaviour. Acceptance: an info on `assertThat` with a fix whose fix range is the
whole line: Alt+Enter at the end of the line offers the fix.

`CI-HLI-051` SHOULD: lazy fixes computed in the background, nearest first, with the Alt+Enter
wait. Acceptance: a lazy fix that sleeps 500 ms is present in the popup opened at 100 ms.

`CI-HLI-052` MUST: unregistering an invoked fix. Acceptance: after applying "Import class",
the popup on the same spot no longer offers it before the next analysis finishes.

### 6.2 Submenu options of a fix

The options are what the user sees when pressing the right arrow on a popup row. They are
computed from the fix and its info (rules in `04-intentions-and-alt-enter.md` section 6).

## 7. Composite infos (several infos at one place)

When several infos overlap one position (hovering, error stripe click, F2 with
"show tooltip"), the UI builds one *composite* info:

1. Remove duplicates by tooltip text.
2. Sort by severity descending, then by tooltip text.
3. Concatenate the tooltips (HTML bodies without their `<html>` wrapper) separated by a thin
   horizontal rule (`<hr>`), and wrap again in `<html>`.
4. Concatenate descriptions per rule 1 of section 2.
5. The composite's range and highlighter come from the first info that has a tooltip.
6. All fixes of all members are attached to the composite.

`CI-HLI-060` MUST: composite construction as listed. Acceptance: a warning and an error on the
same token show the error text first, a rule, then the warning text.

## 8. Error stripe and per-info display

### 8.1 Layers (painting order, lower is painted first)

| Layer (value) | Used by |
|---|---|
| syntax (1000) | lexer colouring |
| caret row (2000) | caret-row background |
| additional syntax (3000) | semantic colouring, symbol-type severity |
| guarded blocks (3500) | |
| weak warning (3750) | WEAK WARNING and TEXT ATTRIBUTES infos |
| warning (4000) | WARNING, and custom severities |
| error (5000) | ERROR and above |
| element under caret (5500) | identifier highlighting |
| console filter (5800), hyperlink (5900), selection (6000) | non-daemon |

A severity maps to a layer by comparing it with the registry: at or above ERROR gives the
error layer, at or above WARNING the warning layer, at or above WEAK WARNING (or TEXT
ATTRIBUTES) the weak-warning layer; injected-fragment and highlighted-reference severities sit
just below the caret row layer; a custom, non-standard severity gets the warning layer; any other
gets the additional-syntax layer. Highlighters of type ERROR, WRONG_REF, WARNING and the two
element-under-caret types stay visible when their text is inside a collapsed fold region (the
placeholder is painted with the highlighter's colour).

`CI-HLI-070` MUST: layers as above, so an error underline paints over a warning background.
Acceptance: an error inside a warning range is visible as a red wave over the warning
background.

### 8.2 Stripe marks

- Every highlighter gets an **error stripe mark colour**: from the forced attributes' stripe
  colour; else from the stripe colour of the severity's attribute key (ERROR/WRONG_REF use the
  `ERRORS_ATTRIBUTES` stripe colour, WARNING the `WARNING_ATTRIBUTES` stripe colour, weak warning
  and INFO the `INFO_ATTRIBUTES` stripe colour, server problems the server key); else from the
  attributes the highlighter itself has. A mark with no colour is not drawn.
- Marks are drawn on the stripe at the proportional vertical position of their line, with a
  minimum thickness setting **"Error stripe mark min height"** (default 2 px; the paint code
  clamps to at most 4 px scaled) in Settings | Editor | General | Error Highlighting.
- Overlapping marks: the one with the higher severity layer is on top.
- **Hover** on a mark (delay setting "tooltips delay", default 500 ms, range 1 to 5000 ms)
  shows the composite tooltip (section 7) of all marks within the hovered pixel band.
  **Click** moves the caret to the mark's navigation offset and shows the tooltip.
- The summary indicator that older versions drew at the top of the stripe is the status widget
  of `01-daemon.md` section 9.

`CI-HLI-071` MUST: stripe colour resolution as above and minimum height setting.
Acceptance: an error at line 100 of 1000 draws a red mark at 10% of the stripe height at least
2 px tall; changing the minimum to 4 px redraws.

`CI-HLI-072` MUST: stripe hover shows the composite tooltip after the tooltips delay; click
navigates. Acceptance: hover over a mark with two infos shows both messages, error first.

### 8.3 Hover tooltip over the text

Hovering a highlighted range shows the info tooltip after the tooltips delay (500 ms). The
tooltip has, in order:

1. the message(s) (HTML), possibly several joined by rules;
2. for inspection infos, an inline link `more…` and the shortcut for *Show Error Description*
   (`Ctrl+F1`, `Cmd+F1` on macOS) in parentheses; activating the link expands the inspection's
   long description;
3. an **actions row**: the highest-priority fix as a link button, labelled with the fix text
   and the shortcut `Alt+Shift+Enter`, then a `More actions…` link with the shortcut `Alt+Enter`;
   while the fix list is being computed the row shows `Loading actions…`;
4. a small cog menu with toggles `Show Quick-Fixes` and `Show Inspection Description`.

The top fix is chosen with the same ordering as the popup (`04-...` section 3) from the info's
own fixes that are available at the info's offset, skipping empty actions.

`CI-HLI-080` MUST: hover tooltip of rule 1 and 3 (message plus top fix plus `More actions…`).
Acceptance: hovering an unresolved `assertThat` shows `Cannot resolve method 'assertThat'`,
a link `Import static method...` with `Alt+Shift+Enter`, and `More actions…`
with `Alt+Enter`; pressing `Alt+Shift+Enter` while the tooltip is shown applies the fix.

`CI-HLI-081` SHOULD: `more…` expansion and cog toggles. Acceptance: toggling off
"Show Quick-Fixes" hides the actions row for later tooltips.

## 9. File-level infos

An info flagged *file level* has the whole file as its range and is not painted in the text.
It is shown as a banner at the top of the editor: an icon for its severity, the message, and
(if it has fixes) a bulb button that opens the Alt+Enter popup for this info. It disappears when
the info is removed by the next analysis. Typical producers: "file is not part of the project",
"no JDK" style messages produced by annotators.

`CI-HLI-090` SHOULD: file-level banner with severity icon, message, fix bulb. Acceptance: an
annotator reporting a file-level warning shows the banner; applying its fix removes it in the
next cycle.

## 10. `ProblemHighlightType` to info type

Inspections register problems with a *highlight type*; the daemon converts it to an info type
and severity so that both the user's profile and the producer's intent are respected.

| `ProblemHighlightType` | Resulting info type | Severity |
|---|---|---|
| GENERIC_ERROR_OR_WARNING | the registry's type for the profile level of the tool (error gives ERROR type, warning gives WARNING type, weak warning gives WEAK_WARNING type, custom gives its own type) | the profile level (per scope) |
| LIKE_UNKNOWN_SYMBOL | if severity is ERROR: WRONG_REF type (red text). If WARNING: the WEAK_WARNING attributes with severity WARNING. Else the registry type for the severity | the profile level |
| LIKE_DEPRECATED | profile severity with `DEPRECATED_ATTRIBUTES` | the profile level |
| LIKE_MARKED_FOR_REMOVAL | profile severity with `MARKED_FOR_REMOVAL_ATTRIBUTES` | the profile level |
| LIKE_UNUSED_SYMBOL | profile severity with `NOT_USED_ELEMENT_ATTRIBUTES` | the profile level |
| ERROR | WRONG_REF type | ERROR, whatever the profile says |
| GENERIC_ERROR | ERROR type | ERROR |
| WARNING | WARNING type | WARNING |
| WEAK_WARNING | WEAK_WARNING type | WEAK WARNING |
| INFO (deprecated) | INFO type | INFO |
| INFORMATION | INFORMATION type | INFORMATION (hidden problem, fix only) |
| POSSIBLE_PROBLEM | POSSIBLE_PROBLEM type | internal (nothing drawn, not reported in batch) |

`CI-HLI-100` MUST: this mapping. Acceptance: a table test over all twelve values.

`CI-HLI-101` MUST: when the profile level of the tool is "do not show" (hidden), no info is
produced for `GENERIC_ERROR_OR_WARNING` problems, but the problem still produces an INFORMATION
info (without squiggle) when it has fixes and the producer asked for it
(`HighlightType` INFORMATION). Acceptance: setting an inspection to "do not show" removes the
squiggle; the fix is still in Alt+Enter on that spot.

## 11. Highlight infos read by other features

| Consumer | What it reads |
|---|---|
| Status widget | counts per severity (CI-DMN-110) |
| Error stripe / hover | stripe colours, composite tooltip |
| F2 | infos with severity of at least "server problem", by navigation offset |
| Problems view (current file tab) | the infos of the file (layout specified in the tool-window spec) |
| Alt+Enter | infos whose fix range contains the caret, plus ERROR infos on the same line (CI-ALT-010..012) |
| Import hints | visible infos that carry an import hint action (`05-auto-import.md`) |
| Project tree | files that contain at least one ERROR info are shown as problem files (the problem-file pass of `01-daemon.md` 3.2) |

`CI-HLI-110` MUST: all consumers above read the same store (the markup model), not
recomputed lists. Acceptance: the Problems view, F2 and the stripe never disagree about the
set of problems; a test compares their outputs after each daemon cycle.

## 12. Acceptance scenarios

**S-HLI-1, unresolved symbol.** File contains `assertThat(x)` with no import. Expect an info:
severity ERROR, type WRONG_REF (red text), description `Cannot resolve method 'assertThat'`,
tool id = the general-pass visitor identity, fix `Import static method...` (TOP priority), fix
range = the line. Counts: `1 error`; stripe mark in the error colour; F2 jumps to it.

**S-HLI-2, deprecated call.** Call a `@Deprecated` method. Expect type DEPRECATED, severity from
the profile (default WARNING in Java profile "Deprecated API usage"), strike-through, stripe mark
in the warning stripe colour, tooltip text of the Java message.

**S-HLI-3, hidden inspection with fix.** Set "Unused declaration" to "do not show": no grey
text, but quick fixes remain available through Alt+Enter on the declaration if the inspection
registers an INFORMATION info; the submenu still lists "Edit inspection profile setting".

**S-HLI-4, two problems, one token.** A warning and an error on the same token: hover shows the
error message, a rule, the warning message; the stripe shows one red mark (error layer on top).

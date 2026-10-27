# 03. Inspections, profiles and suppression

Status: specification, language independent. Requirement prefix `CI-INS`.
Priorities: `MUST` / `SHOULD` / `LATER`; ordering inside each section favours Java/Kotlin/Spring.

Concrete inspection catalogues (which Java checks exist, their messages and fixes) belong to
the language specs. This file specifies the framework they plug into and the behaviours every
inspection inherits: when it runs, how a problem becomes an editor highlight, how profiles and
scopes decide, and how suppression works.

ic: `platform/analysis-api/src/com/intellij/codeInspection/` (`LocalInspectionTool`,
`InspectionProfileEntry`, `ProblemsHolder`, `ProblemDescriptor`, `LocalQuickFix`, `QuickFix`,
`ProblemHighlightType`, `GlobalInspectionTool`, `SuppressionUtil*`),
`platform/analysis-impl/src/com/intellij/codeInspection/ex/` (`InspectionProfileImpl`,
`ToolsImpl`), `platform/analysis-impl/src/com/intellij/profile/codeInspection/`,
`platform/lang-impl/src/com/intellij/codeInsight/daemon/impl/LocalInspectionsPass.java`,
`InspectionRunner.java`.

## 1. Kinds of inspection

| Kind | Runs | Unit of work | Typical example |
|---|---|---|---|
| **Local inspection** | on the fly in the daemon *and* in batch | one file, visited element by element (or whole-file hook) | "Statement can be replaced", "Redundant cast" |
| **Global inspection** | in batch ("Inspect Code") only, unless it exposes a *shared local* counterpart | a whole analysis scope with cross-file reference data | "Unused declaration", "Method can be private" |
| **Highlight visitor / annotator** | always, in the general pass, not in profiles | one element | unresolved symbol, type mismatch (see `01-daemon.md` 6) |
| **External annotator paired with a batch inspection** | on the fly after the general pass | a file | a linter |

Some features exist as a global inspection *and* run on the fly through a shared local tool
(Java "Unused import" and "Unused declaration" are the important ones). On the fly the shared
local part reports what can be known from one file plus index lookups (for unused symbols: is
the name referenced anywhere, answered through the word index); in batch the global tool does
the precise whole-scope computation.

`CI-INS-001` MUST: local inspections, global inspections and the visitor/annotator producers
are three separate extension points. Acceptance: registering a local inspection makes it appear
in the profile tree and run in the daemon; a visitor does not appear in the tree.

`CI-INS-002` LATER: global inspections with a shared local counterpart for on-the-fly use.
Acceptance: "Unused declaration" is a single profile entry that greys out unused private members
on the fly and reports public API too in Inspect Code.

## 2. Identity and metadata of an inspection

Every inspection has these stable attributes (they are what profiles, suppression comments and
settings files refer to, so they must never change once released):

| Attribute | Rule |
|---|---|
| **short name** | Identifier unique across all inspections. Pattern for ids: letters, digits, `_`, `.`, `-`. Default is the class name without the trailing `Inspection` (or `InspectionBase`). It is the settings key in profile files and the key of the display-key registry. |
| **suppress id** | The id used in `noinspection` comments and `@SuppressWarnings`. Defaults to the short name; a different id may be declared (for example short name `Deprecation`, suppress id `deprecation`). |
| **alternative id** | Optional second id that also suppresses (for backward compatibility). |
| display name | Sentence-case text shown in the profile tree and in the "Edit inspection profile setting" row. |
| group path | A list of strings: language group, then categories (for example `Java` then `Probable bugs`). Used for the tree in the settings page and for result grouping. |
| language | The language id it applies to; the inspection runs only for files whose view provider has that language (and, unless the "apply to dialects" flag is off, for dialects of it). Empty means every language. |
| default level | One of the severity names (ERROR, WARNING, WEAK WARNING, INFORMATION as "do not show", TEXT ATTRIBUTES, SERVER PROBLEM, or a custom severity). Default WARNING. |
| enabled by default | Boolean. Disabled inspections can still be found in the settings page and enabled. |
| editor attributes key | Optional text-attributes key used instead of the severity's key when the problem type is GENERIC_ERROR_OR_WARNING (for example `NOT_USED_ELEMENT_ATTRIBUTES` for unused-ness). |
| cleanup tool | Flag: included in "Code Cleanup" runs. |
| project type | Optional: the tool only applies to projects of this type id (for example a framework). |
| description | Long HTML description shown in the settings page and (shortened) via the `more…` tooltip link. Loaded from a resource named after the short name. |
| options | A typed list of option controls (checkbox, integer, string, string list, dropdown, tables) with defaults. The settings page renders them automatically; values are saved in the profile per scope. |
| dumb-aware | Whether it may run during indexing. |
| unfair | Flag for tools that cannot be fair in batch (their on-the-fly counterpart is excluded from "Fix all"). |

`CI-INS-010` MUST: all attributes above are data on the registration (for Archipel: part of the
trait implementation or a manifest), and short name, suppress id and alternative id are
validated against the id pattern at registration. Acceptance: registering an inspection with
the id `bad id` fails with a message naming the inspection.

`CI-INS-011` MUST: two inspections with the same short name are rejected. Acceptance: duplicate
registration is logged as an error and the second one is ignored.

`CI-INS-012` MUST: the default level and enabled-by-default flags are honoured by the built-in
profile (section 6). Acceptance: a new inspection registered at level WARNING and enabled
appears in the "Default" profile with those values.

## 3. The local inspection contract

An inspection implementation supplies one of two entry points:

1. **Visitor** (preferred): given a *problems holder* and the flag *on the fly*, return a
   visitor object that is called for every element of the file (not recursive: the engine feeds
   all elements itself). The visitor registers problems for the element it is called on and
   must keep the reported range inside the element, to avoid flicker. It must be thread-safe
   because the engine may call it concurrently for different elements. Returning an empty
   visitor means "not applicable now".
2. **Whole-file check**: given the file and the flag *on the fly*, return a list of problem
   descriptors. Discouraged because nothing is shown before the whole file is analysed.

Further hooks:

- *runs for whole file*: return true to be called for every element of the file after every
  change, not only for the dirty range. Needed by inspections whose result for one element
  depends on text far away ("field can be local"). Costly; use rarely.
- *is available for file*: veto per file (for example by file name or framework presence).
- *on the fly flag*: the same tool runs in both modes. In batch mode all expensive checks are
  allowed. In on-the-fly mode, expensive checks must be omitted or replaced by a cheaper
  approximation, and "possible problem" markers can be registered for skipped places.
- *session*: per-run object that lets a tool keep state across elements and ask for the
  priority/visible range.
- *quick fixes* come with each descriptor (section 4); the tool never edits code from the
  visitor.

`CI-INS-020` MUST: visitor and whole-file entry points, the `runs for whole file` flag, the
`is available for file` veto, and the `on the fly` flag. Acceptance: a test inspection whose
visitor reports on every `TODO` comment runs in the daemon (on-the-fly = true) and in batch
(false) with the same descriptors.

`CI-INS-021` MUST: visitors are never required to be recursive, and the engine feeds each
element exactly once per run per tool. Acceptance: a counting visitor sees each element once.

`CI-INS-022` MUST: descriptors reported outside the file being inspected are rejected with an
error naming the tool (injected fragments and "externally defined" elements may redirect).
Acceptance: an inspection that registers a problem on another file's element is logged as
a plugin error and the problem is dropped.

`CI-INS-023` MUST: a thrown exception or an index-not-ready exception inside a tool is caught;
the tool contributes nothing for that element, other tools continue. Acceptance: an
inspection that throws on every element does not stop other inspections.

## 4. Problems holder and problem descriptor

### 4.1 `ProblemDescriptor` (what one problem is)

| Field | Meaning |
|---|---|
| start element, end element | the element(s) the problem is anchored to (usually one element) |
| range in element | optional sub-range relative to the element (the highlighted text); default the full element range |
| description template | message with placeholders (`#ref`, `#loc`, `#end`, `#treeend`; see `02-highlight-info.md` section 2) |
| tooltip template | optional; default equal to the description template |
| highlight type | a `ProblemHighlightType` (`02-highlight-info.md` section 10) |
| quick fixes | ordered list of `LocalQuickFix` objects (may be empty) |
| after end of line | draw after the end of the line (for empty ranges) |
| show tooltip | whether the tooltip with the `more…` link is built |
| enforced text attributes | optional key overriding the type's key |
| problem group | optional grouping for custom suppress submenus |
| hint action | optional extra action shown as an editor hint (used by import fixes) |
| reporter id | optional: report under another tool's id (a shared checker reporting for several tools) |

### 4.2 `ProblemsHolder` (how a tool reports)

Registration forms the holder must offer:

1. element, message, optional fixes (highlight type GENERIC_ERROR_OR_WARNING);
2. element, message, highlight type, optional fixes;
3. element, optional sub-range, message, highlight type, optional fixes;
4. a ready `ProblemDescriptor`;
5. a *reference*: element and range come from the reference; if the reference itself provides
   quick fixes (a "fix provider" reference), they are used; the default message is the
   unresolved-reference message (`Cannot resolve symbol '<name>'`), type LIKE_UNKNOWN_SYMBOL;
6. a *possible problem* marker on an identifier (type POSSIBLE_PROBLEM, silent).

The holder exposes: the file, the project, the on-the-fly flag, result count and results.
Problems registered outside the file are redirected for "externally defined" elements (a
template language mapping to its host) by replacing the message with a text that names the
original file and offset, otherwise rejected (CI-INS-022).

`CI-INS-030` MUST: the six registration forms above. Acceptance: unit test per form producing
the expected descriptor fields.

`CI-INS-031` MUST: a descriptor that has no fixes and no enforced attributes gets an *empty
action* at conversion (section 5, step 8). Acceptance: see CI-ALT-040.

### 4.3 `LocalQuickFix` / `QuickFix`

| Member | Meaning |
|---|---|
| name | text shown in the popup (defaults to the family name) |
| family name | stable text identifying the *kind* of fix; used to group "Fix all" and to compare fixes |
| apply fix(project, descriptor) | performs the change; runs in a command, usually in a write action |
| starts in write action | false when the fix shows a dialog and does its own write action |
| available in batch mode | default true; false for fixes that need user interaction |
| generate preview | produces an intention preview (see `04-intentions-and-alt-enter.md` section 8) |
| ranges to highlight | optional list of ranges the popup highlights while the row is selected |

A quick fix is wrapped into an intention-like action when attached to the editor highlight so
that the popup, tooltip and preview treat it uniformly (CI-FIX-020).

`CI-INS-040` MUST: name versus family name semantics. Acceptance: two fixes with the same family
and different names ("Remove 'a'", "Remove 'b'") both appear, and a single "Fix all" row can
combine them (CI-ALT-050).

## 5. Running local inspections in the daemon (algorithm)

Per run of the *local inspections pass* for one file, restricted to the pass's dirty range and
with the visible range as priority range:

1. **Pick the tools.** From the active profile, all local tools such that:
   a. enabled for this file (profile and scope, section 6.3);
   b. the effective level for this file is not "do not show" and not "text attributes only"
      (those levels never run on the fly except for tools that registered a hidden fix);
   c. the tool's language matches the file's languages (or is empty);
   d. the tool's project type matches (when declared);
   e. in dumb mode: the tool is dumb-aware; the others are skipped and their old highlights
      are kept (CI-DMN-053);
   f. the tool is not suppressed for the whole file;
   g. the tool is available for this file;
   h. the highlighting level of the file is at least "inspections" (`01-daemon.md` section 7).
2. **Create one visitor per tool** with a fresh holder for the run. Tools whose visitors are
   empty are dropped.
3. **Divide the elements.** The file's elements (also those of injected fragments and of all
   language roots) are split into "inside the priority range" and "outside". Tools that run for
   the whole file get the whole-file lists instead of the dirty-range lists.
4. **Order.** Visit "inside" first, then "outside". Tools are scheduled by past latency
   (tools that were fast last time start first; the order is remembered per file) and elements
   by past *fertility* (elements that yielded problems before are visited earlier), so first
   results arrive early.
5. **Parallelism.** Different tools run in parallel; one tool's visitor is never entered
   concurrently with itself. Each tool visits its element list in order.
6. **Element-kind dispatch.** A tool may declare which element kinds it is interested in; the
   engine builds one dispatch table per file so that one tree walk feeds all tools and a tool
   is only called for matching elements.
7. **Report incrementally.** After each element a tool produced descriptors for, convert them
   immediately (step 8) and hand the infos of that element to the markup updater (so the user
   sees early results, and recycling per element works).
8. **Convert each descriptor to a `HighlightInfo`:**
   a. skip if the tool is suppressed at the element (section 8);
   b. determine the *level* from the profile for this tool and file (scope rules, 6.3);
   c. map highlight type and level to info type and severity (`02-highlight-info.md` section 10);
   d. skip if the tool (or the problem group's tool) is not enabled for the file;
   e. render description and tooltip (templates); for tooltips of descriptors with
      `show tooltip` add the `more…` link and the shortcut text, unless the tool has an
      explicitly empty static description;
   f. choose text attributes: enforced attributes first; else for GENERIC_ERROR_OR_WARNING the
      profile's editor-attributes key for the tool; else the info type's key;
   g. an INFORMATION-level info with no attributes but with fixes gets a non-empty attribute
      placeholder so that the "hide empty info" filter does not drop it;
   h. set range (sub-range, after-end-of-line, file-level if the range is the whole file);
   i. register the fixes: each `LocalQuickFix` becomes an intention-like action; an optional
      hint action is added; if there are no fixes and no enforced attributes and this
      (range, tool) pair has not yet got an empty action in this run, add the *empty action*
      whose text is the tool's display name;
   j. set tool id = the tool that ran, group = the inspections pass, run the post filters;
   k. for infos from injected fragments, map the range to host offsets by intersecting with
      editable fragments and creating one info per piece.
9. **Finish.** Remove infos of tools that did not run (disabled, no longer applicable) except
   those of non-dumb-aware tools in dumb mode; remove warnings that sit inside error ranges of
   the same file (injected-language case); run the *redundant suppression* tool last when
   enabled (section 8.5).

`CI-INS-050` MUST: steps 1 to 3 and 7 to 9, in particular the tool selection filters, the
priority-range-first order and incremental reporting. Acceptance: with two tools, one slow, the
fast tool's problems appear before the slow tool finishes.

`CI-INS-051` MUST: a tool disabled in the profile removes its highlights within one daemon
cycle. Acceptance: disable "Unused import" in the profile; grey-out disappears.

`CI-INS-052` MUST: one visitor instance per tool per run, never entered concurrently.
Acceptance: a thread-unsafe visitor passes a stress test of 10 runs.

`CI-INS-053` SHOULD: step 4 latency and fertility ordering. Acceptance: on a file with 50
tools the second run produces the first problem earlier than the first run (benchmark).

`CI-INS-054` SHOULD: step 6 element-kind dispatch. Acceptance: a tool declaring interest in
`string literal` is called only for string literals (instrumented).

`CI-INS-055` MUST: the inspection pass never runs for files at highlighting level Syntax or
None. Acceptance: see S-DMN-6.

## 6. Profiles

### 6.1 Model

An **inspection profile** is a named set of *tool states*:

- For every inspection short name: a tool entry with
  - *enabled* (overall switch),
  - a **default state**: enabled-by-default, level, optional editor-attributes override and the
    option values,
  - an ordered list of **scope states**: scope name, enabled, level, optional editor-attributes
    override and *its own copy of the option values*.
- A profile has a **name**, an optional **base profile** (the built-in defaults that it
  overrides; only differences from the base are persisted), and a **kind**: *application* (IDE)
  or *project*.
- Built-in profiles: `Default` (application) with every inspection at its registered defaults;
  `Project Default` (project) which starts as a copy of `Default`. A project always has one
  *current* profile, by default `Project Default`.
- Which profile is current is stored per project (`profiles_settings`: the project profile name
  and a flag "use project profile").
- Changing, activating, adding or deleting a profile or opening a project's profile list
  restarts the daemon for all files ("profile changed" in CI-DMN-060).

### 6.2 Persistence

IntelliJ stores profiles as XML (`<project>/.idea/inspectionProfiles/<Name>.xml` with
`profiles_settings.xml` next to them; application profiles in the config directory). The
structure is: one `profile` element with `version`, an option `myName`, optional base-profile
reference, and one `inspection_tool` element per tool whose state differs from the base, with
attributes `class` (short name), `enabled`, `level`, `enabled_by_default`, optional
`editorAttributes`, child `scope` elements (`name`, `level`, `enabled`, `editorAttributes`),
and the tool's own option elements.

Archipel stores TOML (`docs/analysis/daemon.md`, `.archipel/inspection-profiles/<name>.toml`)
with the same logical content: `base`, `[tools.<id>] enabled, level, editor_attributes`,
`[[tools.<id>.scopes]] scope, enabled, level`, and `[tools.<id>.options]`. That layout is
accepted. Importing and exporting IntelliJ XML profiles is `LATER`.

`CI-INS-060` MUST: the profile model of 6.1 with base profile, default state, ordered scope
states, per-scope option copies, application and project kinds, current profile per project.
Acceptance: unit test: base enables tool A at WARNING; the profile overrides A to ERROR for
scope `tests` and disables tool B; the effective states for a main and a test file differ as
expected.

`CI-INS-061` MUST: persist only differences from the base, in the TOML layout above, and reload
losslessly. Acceptance: round trip of a profile with 3 overridden tools and one scope override
reproduces the same effective states and the file contains only those 3 tools.

`CI-INS-062` MUST: built-in `Default` and project `Project Default`; the current profile is
`Project Default` for new projects. Acceptance: a fresh project reports the current profile
name `Project Default` and no profile file on disk until the user changes something.

`CI-INS-063` SHOULD: copy between IDE and project, duplicate, rename, delete (not the last),
restore defaults. Acceptance per operation via the settings page of SET-ED-016.

`CI-INS-064` LATER: import and export of IntelliJ XML profile files. Acceptance: importing a
`Project_Default.xml` with 20 tools yields the same enabled set and severities.

### 6.3 Resolution: which state applies to a file

For tool T and file F (injected fragments use the top-level host file):

1. If T's overall enabled flag is false: disabled.
2. Walk T's scope states **in list order**. For each scope state whose *scope* contains F, take
   that state (enabled flag, level, option copy, attributes override). **The first matching scope
   wins.** (Not the most specific one: order matters, and the settings page lets the user
   reorder.)
3. If no scope matches, use the default state.

Predefined scopes (names are persisted): `All Places`, `Project Files`, `Non-Project Files`,
`Scratches and Consoles`, `Production`, `Tests`; the user can define named scopes with
patterns (`file:src/test//*` style) in Settings | Appearance & Behavior | Scopes. Scope
membership for `Tests` and `Production` comes from the module's source/test root flags.

The *level* resolution also applies the *severity registry*: a level name that is unknown to the
registry (a deleted custom severity) falls back to WARNING when reading.

"Batch availability": in batch runs a state at level "do not show" or "text attributes" is
skipped (these levels exist for fix-only or colour-only on-the-fly effects).

`CI-INS-070` MUST: first-matching-scope-wins resolution and default fallback. Acceptance:
scopes `[tests: ERROR, all: WARNING]` give a test file ERROR; reversing the order gives WARNING.

`CI-INS-071` MUST: predefined scopes `All Places`, `Project Files`, `Non-Project Files`,
`Production`, `Tests`, `Scratches and Consoles`; user-defined scopes by glob. Acceptance: a
scope `src/main/**/*Controller.java` matches only controllers.

`CI-INS-072` MUST: each scope state carries its own copy of option values. Acceptance: setting
"Maximum line length" to 120 for scope `tests` and 100 elsewhere yields different thresholds
per file.

`CI-INS-073` MUST: the effective level feeds the HighlightInfo severity (so counts, F2 and the
stripe agree) and the batch availability rule. Acceptance: tool at "do not show" in the profile
does not appear in Inspect Code results.

### 6.4 Settings page (reference)

The page is specified in `docs/spec/settings.md` (SET-ED-016). Framework requirements it
relies on: the tree is built from group paths; selecting a tool shows the description and the
scope table; the severity combo lists the registry's applicable severities plus "Edit
Severities…"; the options panel is rendered from the tool's option definitions; every change
sets the profile "modified" state until Apply; Apply restarts the daemon.

`CI-INS-080` SHOULD: auto-generated options UI from typed option definitions (checkbox,
integer with range, string, list of strings, dropdown, table). Acceptance: an inspection
declaring `max line length: int 1..1000 default 120` shows a numeric field and persists it.

## 7. Quick fixes at the framework level (summary; details in `04-...`)

- Every attached fix exists in two forms: the original `LocalQuickFix` for batch, and an
  intention-like wrapper for the editor popup.
- "Fix all 'X' problems in file" and "Apply all 'Y' fixes in file" are generated from the
  inspection tool and the fix family respectively (CI-ALT-050, CI-ALT-051).
- In batch, a fix is applied to many descriptors in one command (one undo step per run or per
  file), skipping fixes that report `available in batch = false`.

## 8. Suppression

A problem is **suppressed** when a suppression marker applies to the element where the problem
is reported. The check happens when converting descriptors (step 8a) and, as a cheaper filter,
when selecting tools (file-level suppression).

### 8.1 Comment form

- Marker text (case-sensitive): `noinspection` followed by one or more whitespace and a comma
  separated list of suppress ids (or alternative ids). Spaces around commas are allowed.
  Example: `//noinspection unchecked, rawtypes`. The id `ALL` (case-insensitive) suppresses all
  tools.
- The comment must be a **line comment that is the immediate previous sibling of the statement
  or declaration** (whitespace and newlines between are ignored). It applies to that whole
  statement or declaration, including nested elements. A trailing comment on the same line
  after the code is **not** a recognised form.
- The id list may be followed by an arbitrary trailing word and text ("noinspection Foo reason").
- Which element kinds count as "statement" or "declaration" is a per-language decision, made by
  the language's *suppressor* (see 8.4).
- Block comments are accepted where the language defines them as suppression carriers (for
  example HTML/XML comments before a tag).

### 8.2 Annotation form (Java, Kotlin)

- `@SuppressWarnings("id")`, `@SuppressWarnings({"id1", "id2"})` on a declaration (class,
  method, field, parameter, local variable, module declaration) suppresses those ids within it.
  The value may be a string literal or a reference to a constant string. A single string may
  itself contain several ids separated by commas or spaces. `ALL` is accepted in any letter case.
  Ids are compared with the suppress id and the alternative id.
- Kotlin uses `@Suppress("id")`; ids are the same suppress ids (the Kotlin plugin may accept
  compiler warning names too).
- Javadoc form for Java 1.4 code style: a doc tag `@noinspection id` on the declaration's doc
  comment.
- A suppression on an enclosing class applies to all members; on the file's package statement or
  module declaration applies to the whole file.

### 8.3 File and project level

- File-level: language-defined form (Archipel: a comment `noinspection-file <ids>` in the file
  header before the first declaration, as in `daemon.md`) suppresses the ids for the whole file.
  Not an IntelliJ-wide form: IntelliJ uses per-language mechanisms (class annotation or a
  top-of-file marker where defined). The Archipel form is allowed because it only has to
  round-trip within the tool.
- Scope-level suppression is the profile's "disabled in scope" (6.3), not a comment.
- A global setting "Suppress warnings" (default on) enables the `@SuppressWarnings` mechanism for
  Java; when off, those annotations are ignored.

### 8.4 The suppressor extension

A per-language `InspectionSuppressor` provides:
- `is suppressed for(element, tool id)`: true if any marker of 8.1 to 8.3 applies;
- `suppress actions(element, tool id)`: the fixes that *add* a marker (see 8.6).

The default suppressor handles comments for languages that have line comments. A language
adds its own (annotations, doc tags, XML comments). Multiple suppressors are all consulted;
if any says suppressed, the problem is suppressed.

### 8.5 Redundant suppression

An optional inspection ("Redundant suppression", default on, level WARNING) runs after all other
tools in the same pass and reports a suppression marker (id in a `noinspection` comment or
annotation) for which the named tool reports nothing at that place. It must not report when the
tool could not fully check the place (the *possible problem* marker, or the tool is disabled,
or the tool is of a kind that cannot run on the fly). It offers the fix "Remove suppression".

### 8.6 The "Suppress" quick fixes (offered in the Alt+Enter submenu of a problem)

For the tool T of the info at the caret the popup submenu contains suppress rows chosen by the
language's suppressor. The standard texts are:

| Row text | Effect |
|---|---|
| `Suppress for statement` | inserts `//noinspection T` on its own line directly above the enclosing statement, or appends `, T` to an existing marker above it |
| `Suppress for statement with comment` | Java variant that inserts the marker as a comment |
| `Suppress for class` | adds `@SuppressWarnings("T")` to the enclosing class, or appends to an existing annotation's list |
| `Suppress for method` | the same on the enclosing method |
| `Suppress for field` | the same on the field |
| `Suppress for parameter` | the same on the parameter |
| `Suppress for member` | the same on the member (generic wording) |
| `Suppress for file` | the language's file-level form |
| `Suppress all inspections for class` | `@SuppressWarnings("ALL")` |
| `Suppress for 'Tests' scope` | a tool-specific shortcut that adds a profile scope override (appears for a few Java inspections) |

Only the rows whose target element exists at the caret are offered (no "for method" outside a
method). A suppress row on an injected fragment is offered for the host or the fragment
depending on where the language says the marker must go.

`CI-INS-090` MUST: the comment form of 8.1 exactly (previous-sibling line comment; id list;
ALL; no trailing form). Acceptance: `//noinspection unused` above `int x = 1;` suppresses
`unused` for that statement only; `int x = 1; //noinspection unused` suppresses nothing.

`CI-INS-091` MUST: `@SuppressWarnings` and `@Suppress` forms for Java and Kotlin with `ALL`,
arrays and scopes of 8.2, honouring the Java "Suppress warnings" setting. Acceptance: a method
annotated `@SuppressWarnings("unchecked")` hides the "Unchecked call" warning inside it but not
in the next method.

`CI-INS-092` MUST: a comment edit adding or removing a marker re-analyses the enclosing
scope (CI-DMN-032). Acceptance: see there.

`CI-INS-093` MUST: the suppress rows of 8.6 for statement, class, method, field, file in the
Alt+Enter submenu, generated from the language suppressor and the tool's suppress id.
Acceptance: choose `Suppress for method` on a flagged call: the enclosing method gains
`@SuppressWarnings("unchecked")` (or the id appended) and the warning goes away.

`CI-INS-094` SHOULD: the redundant-suppression inspection of 8.5 with "Remove suppression".
Acceptance: `@SuppressWarnings("rawtypes")` on a method with no raw type is flagged; after the
fix the annotation is removed (or the id removed from the list).

`CI-INS-095` LATER: `Suppress for 'Tests' scope` style tool-specific shortcuts.

## 9. Batch: "Inspect Code"

### 9.1 Flow

1. **Action** `Inspect Code…` (Code menu; no default shortcut) opens the dialog **Specify
   Inspection Scope**: options `Whole project`, `Module '<name>'`, `Uncommitted files`
   (with a change-list chooser when several), `Custom scope:` (combo of defined scopes), a
   `Include test sources` checkbox, an `Inspect injected code` checkbox, and an *Inspection
   profile* chooser with a "…" button to the profile editor. A variant `Run Inspection by Name…`
   (`Ctrl+Alt+Shift+I`) asks for a single inspection (search-as-you-type over display names) and
   then shows the same scope dialog. A variant `Analyze | Inspect Code…` on a selected file or
   directory skips the scope dialog's "whole project" default and pre-selects the selection.
2. **Run** in a background task "Inspecting code..." with progress and cancel; the engine
   walks the files of the scope, parses on demand, runs all *enabled* local tools with
   `on the fly = false`, and global tools (two phases: collect references, then evaluate).
   Files are processed in parallel with a bounded pool; memory is capped by an LRU of parsed
   trees. Tools at "do not show" level are skipped. Global tools run after all local tools
   have visited all files because they need whole-scope data.
3. **Results view**: a tool window titled `Inspections on <scope description>` (tab title
   `Inspection Results`; the profile is shown as `'<name>' profile`) with a tree:
   `root` > (optional group by Severity) > *language/group path* > *inspection* >
   *file or directory (module/package grouping toggle)* > *problem* (icon by severity, message
   with the `#ref` rendered, `line:col`). Counts appear in each node. A right-hand panel shows
   the inspection description, the code preview of the selected problem with the range
   highlighted, and the list of its quick fixes with `Apply`/`Show Quick Fixes`.
4. **Toolbar**: `Rerun`, `Go to Next Problem`, `Go to Previous Problem` (F2-like), `Expand all`,
   `Collapse all`, `Group By` (Directory, Severity), `Filter resolved items`, `Export` (HTML, XML, SARIF), `Settings…` (opens the profile page), `Suppress`
   popup for the selected node, `Show Quick Fixes` (apply the selected fix to all selected
   problems).
5. **Applying fixes in batch**: selecting a fix for a group node applies it to all contained
   problems in one go. Fixes with `available in batch = false` are not offered for groups. The
   result is one undoable command per file group; a summary notification states how many
   problems were fixed; files that need write permission prompt like any edit.
6. **"No problems"**: if nothing was found, an information message `Code inspection did not
   find anything to report. N file(s) processed in '<scope>'.` with a link `Repeat the analysis
   with test sources` when tests were excluded.
7. **Code Cleanup** (`Code | Code Cleanup…`) runs only tools flagged as cleanup tools and applies
   their fixes without showing the tree, in one undoable operation, asking for a scope like the
   inspect dialog; "Fix all" rows in the popup use the same fix machinery for the current file.

### 9.2 Data model of results

A result record is: tool short name, severity (level at the problem's file), file, range, message,
quick fix list (with family and name), "is suppressed" flag (suppressed problems are not
reported), and a stable id for incremental re-run. The Problems tool window and the editor read
the *same* descriptor-derived `HighlightInfo` for the current file; the batch view is a
separate, persisted result set for the whole scope.

### 9.3 Global inspection tools (`LATER`)

- A global tool receives the analysis scope, a reference graph (declarations, references,
  inheritance) and a sink for problems. It may mark declarations as "entry points" (a main
  method, a servlet, a test method, a class annotated by a framework annotation) so that they
  are not reported as unused; entry points come from an extensible provider and from a user
  list.
- Examples that matter for Java: Unused declaration (with options for entry points, "check
  parameters", "check class initializers"), Method can be private/final, Class can be package
  private.

`CI-INS-100` MUST: a headless "inspect these files with this profile" API returning the
descriptor list, used by tests and by the scenario harness. Acceptance: running it on a fixture
file returns the same problems the daemon would show (modulo on-the-fly-only differences).

`CI-INS-101` SHOULD: `Inspect Code…` with the dialog of 9.1.1, background run, and the results
tree of 9.1.3 with navigation, grouping by severity and by directory, rerun and export to HTML.
Acceptance: run on a fixture project with 3 known problems; the tree shows them grouped under
their inspections; double-click opens the editor at the range; `Rerun` repeats.

`CI-INS-102` SHOULD: batch fix application of 9.1.5 with a single undo step per file group.
Acceptance: apply "Remove unused import" to a group of 10 files; each file changes; one Undo per
file restores it.

`CI-INS-103` SHOULD: `Run Inspection by Name…`. Acceptance: type "unused imp", choose "Unused
import", scope dialog appears, results contain only that inspection.

`CI-INS-104` SHOULD: Code Cleanup of 9.1.7. Acceptance: running cleanup on a file with a
redundant cast and an unused import applies both fixes.

`CI-INS-105` LATER: global inspections and entry points of 9.3 (Unused declaration for public
API). Acceptance: a public method used only from test code is reported with scope production;
a method annotated `@Bean` (Spring entry point) is not.

## 10. Starter set for the Java/Kotlin/Spring MVP (framework-level checks)

The following inspections exercise every part of the framework and are the first ones to
implement; the Java spec owns their precise conditions and messages.

| Short name | Group | Level (default) | Highlight type / attributes | Notes |
|---|---|---|---|---|
| `unused` (Unused declaration) | Declaration redundancy | WARNING | GENERIC with `NOT_USED_ELEMENT_ATTRIBUTES` | global with on-the-fly shared part |
| `UNUSED_IMPORT` (Unused import) | Imports | WARNING | GENERIC with `NOT_USED_ELEMENT_ATTRIBUTES`; on the fly: message `Unused import statement`, fixes `Remove unused imports` and, only while the setting is off, `Enable 'Settings \| Editor \| General \| Auto Import \| Optimize imports on the fly'` (LOW); batch (Inspect Code): message `Unused import <ref>`, fix `Delete unnecessary import` | produced by the general-pass visitor, not by a plain local tool; enables the optimize-imports flow (`08-...`) |
| `MISSORTED_IMPORTS` (Unsorted imports) | Imports | INFORMATION | information with fix `Optimize imports` | on-the-fly optimize trigger |
| `Deprecation` (suppress id `deprecation`) | Code maturity | WARNING | LIKE_DEPRECATED | |
| `MarkedForRemoval` | Code maturity | ERROR | LIKE_MARKED_FOR_REMOVAL | |
| `UnusedAssignment`, `ConstantValue` | Probable bugs | WARNING | GENERIC | data-flow based |
| `NullableProblems` | Probable bugs | WARNING | GENERIC | annotation based |
| `WrongPackageStatement` | Probable bugs | ERROR | GENERIC | quick fixes `Move to package`/`Set package` |
| `RedundantSuppression` | General | WARNING | GENERIC | section 8.5 |

`CI-INS-110` MUST: the Java default profile enables, at the listed levels, at least the
inspections above that are implemented, with the same short names. Acceptance: a profile dump
lists them with levels.

## 11. UX acceptance scenarios

**S-INS-1, unused import.** File has `import java.util.List;` that is not used. Expected: the
import text is grey (attributes `NOT_USED_ELEMENT_ATTRIBUTES`), tooltip `Unused import statement`,
stripe mark in the warning colour; Alt+Enter on the import shows `Remove unused imports`, a LOW
row `Enable 'Settings | Editor | General | Auto Import | Optimize imports on the fly'` (while that
setting is off). The submenu of `Remove unused imports` (Right arrow) contains `Edit inspection
profile setting`, `Run inspection on…`, `Disable highlighting, keep fix`, `Disable inspection`
and the suppress rows the language offers; choosing `Disable inspection` turns the inspection off in the project
profile and the grey-out disappears within one cycle.

**S-INS-2, per-scope severity.** Set `Deprecation` to ERROR for scope `Production` and
WARNING in `Tests`. A deprecated call in `src/main` is red with strike-through; in `src/test`
yellow. The status widget counts follow.

**S-INS-3, suppression.** Put `//noinspection unchecked` above `List<String> l = (List<String>) o;`:
the "Unchecked cast" warning disappears; delete the comment: it returns after one cycle. Use
`@SuppressWarnings("unchecked")` on the method: same result for the whole method.

**S-INS-4, hidden problem with fix.** Set an inspection to "No highlighting (fix only)": no
squiggle; Alt+Enter on that spot still lists its fix (CI-HLI-101).

**S-INS-5, Inspect Code.** `Code | Inspect Code…`, choose `Whole project`, `Include test
sources` off, profile `Project Default`. Expected: progress in the status bar; a tool window
`Inspections on Project 'demo'`; problems grouped by language then inspection; selecting one
shows its description and the code preview with the range highlighted.

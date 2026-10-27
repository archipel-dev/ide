# Editor spec 04: code folding

Status: specification, language independent except section 6 (Java). Requirement prefix `ED-FOLD`.
Priorities: `MUST` / `SHOULD` / `LATER`.

Specifies the fold region model, how regions are produced and refreshed (framework level; each
language supplies its own builder, see `platform/08-language-extension-points.md` `PLAT-LANG` for
the extension point), the folding commands and their exact selection rules, default fold states and
their settings, custom (comment based) regions, the Java regions and placeholders, interaction with
caret movement and editing, and persistence of the user's fold state. The gutter drawing of fold
markers is in `07-editor-appearance-and-settings.md`.

Executable acceptance cases are in `cases/04-folding.cases.md`. In those files a **collapsed**
region is written `<fold text="PLACEHOLDER">covered text</fold>` and an **expanded** region
`<fold text="PLACEHOLDER" expanded>covered text</fold>`; `keys: open file` means the file is opened
for the first time and the initial folding pass has run. Cases are normative.

ic: `platform/editor-ui-api/src/com/intellij/openapi/editor/{FoldRegion,FoldingModel}.java`,
`platform/core-api/src/com/intellij/lang/folding/{FoldingBuilder,FoldingBuilderEx,FoldingDescriptor,
CustomFoldingBuilder,CustomFoldingProvider,CompositeFoldingBuilder,LanguageFolding}.java`,
`platform/core-api/src/com/intellij/codeInsight/folding/CodeFoldingSettings.java`,
`platform/foldings/src/com/intellij/codeInsight/folding/impl/{CodeFoldingManagerImpl,CodeFoldingPass,
FoldingUpdate,UpdateFoldRegionsOperation,DocumentFoldingInfo,EditorFoldingInfo,CollapseSelectionHandler,
CollapseBlockHandlerImpl,CollapseExpandDocCommentsHandler}.java`,
`platform/foldings/src/com/intellij/codeInsight/folding/impl/actions/*`,
`platform/lang-impl/src/com/intellij/lang/customFolding/{VisualStudio,NetBeans}CustomFoldingProvider.java`,
`java/java-psi-impl/src/com/intellij/codeInsight/folding/impl/{JavaFoldingUtil,JavaFrontendFoldings,
JavaBackendFoldings,CommentFoldingUtil,JavaCodeFoldingSettingsBase}.java`,
`platform/editor-ui-api/src/com/intellij/openapi/editor/actions/EditorActionUtil` (movement),
`platform/platform-resources/src/keymaps/$default.xml`,
`platform/ide-core/resources/messages/ApplicationBundle.properties`,
`platform/lang-api/resources/messages/CodeInsightBundle.properties`,
`platform/platform-resources-en/src/messages/ActionsBundle.properties`.

## 1. Model

A **fold region** is a range of the document that can be shown as a single *placeholder* text.

| Property | Meaning |
|---|---|
| Range | Start and end offset; maintained by the document like a range marker, valid until the text is edited so that it collapses. |
| Placeholder | The text drawn instead of the range while collapsed. |
| Expanded | `true` shows the real text. Collapsed regions are drawn as a single unit; the caret cannot be inside. |
| Group | Regions of one group expand and collapse together (used for the one-line method fold, which consists of two regions). |
| Never expand | A region that cannot be expanded (it acts as a custom inline replacement); caret and selection treat it as an atomic unit. |
| Nested | Regions form a forest: a region contains whole regions or is disjoint. Two regions may share the same start but not partially overlap. |

A region produced by a **language builder** is *generated*; a region created by the user (Fold
Selection) is *user-defined*. Generated regions are re-synced with the syntax tree after edits
(section 3); user-defined ones are left alone.

## 2. Commands and default shortcuts

| Action (ID) | Shortcut (Windows/Linux) | Text |
|---|---|---|
| ExpandRegion | Ctrl+Numpad+ (also Ctrl+=) | Expand |
| CollapseRegion | Ctrl+Numpad- (also Ctrl+-) | Collapse |
| ExpandRegionRecursively | Ctrl+Alt+Numpad+ (also Ctrl+Alt+=) | Expand Recursively |
| CollapseRegionRecursively | Ctrl+Alt+Numpad- (also Ctrl+Alt+-) | Collapse Recursively |
| ExpandAllRegions | Ctrl+Shift+Numpad+ (also Ctrl+Shift+=) | Expand All |
| CollapseAllRegions | Ctrl+Shift+Numpad- (also Ctrl+Shift+-) | Collapse All |
| ExpandToLevel1 to 5 | Ctrl+Numpad* then 1 to 5 | Expand to level |
| ExpandAllToLevel1 to 5 | Ctrl+Shift+Numpad* then 1 to 5 | Expand all to level |
| CollapseSelection | Ctrl+. | Fold Selection / Remove Region |
| CollapseBlock | Ctrl+Shift+. | Fold Code Block |
| GotoCustomRegion | Ctrl+Alt+. | Custom Folding... |
| ExpandCollapseToggleAction | none | Toggle Folding |
| ExpandDocComments / CollapseDocComments | none | Expand Doc Comments / Collapse Doc Comments |

The Ctrl+Numpad+ and Ctrl+Numpad- keys are also bound to `ExpandAll`/`CollapseAll` in trees and
`ExpandExpandableComponent`/`CollapseExpandableComponent` in other components; the editor context
decides. Where the editor action and the generic action compete, the editor action wins when an
editor has focus.

## 3. Requirements: producing and refreshing regions

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-FOLD-001 | MUST | A language registers a **folding builder**; builders of several languages of one file (multi-language file) and of injected fragments all contribute (`CompositeFoldingBuilder`). The builder is asked, for the tree root, document and a "quick" flag (quick = no resolve, no index): a list of **folding descriptors** with range, placeholder text, optional group, optional "never expand" flag and a "collapsed by default" answer. The builder may be dumb-aware; non-dumb-aware builders are not run in dumb mode and the previous regions stay. | A test builder returns two descriptors; two regions appear. |
| ED-FOLD-002 | MUST | The folding pass runs after the file is opened (first pass uses the quick flag when the index is not ready, a later pass without it), and again after the document changes and is committed (the daemon's folding pass, `CI-DMN`). It must be cancellable and must not block typing; regions are applied in the UI thread in one batch. | Typing in a large file stays responsive; regions are updated after a pause. |
| ED-FOLD-003 | MUST | **Update rules** for an existing editor: (1) existing regions that still match a new descriptor (same range, same placeholder, same group, same "never expand") are kept with their current expanded state; (2) existing generated regions with no matching descriptor, an invalid range or a stale signature are removed; (3) new descriptors become regions; their state is: the state the removed region of the same range had, else "collapsed by default" from the builder (a language setting), except a region that **contains the caret strictly inside** (caret not at its start) is created expanded. (4) grouped regions are all kept or all removed. | Collapse a method, type elsewhere: the method stays collapsed. Add a method: it follows the default. |
| ED-FOLD-004 | MUST | A region that is collapsed and whose text is edited so that its range becomes empty or invalid disappears. A region whose builder marks it **removable when collapsed** (the Java import list) is dropped if the user deletes its whole content while collapsed. | Delete all collapsed imports: the region is gone and the lines are removed. |
| ED-FOLD-005 | MUST | Descriptor rules shared by builders: a descriptor covering the whole file is ignored; a descriptor whose collapsed form would be no shorter than the text (range length at most the placeholder length when single-line regions are allowed), or spanning a single line when single-line regions are not allowed, is ignored; ranges with syntax errors nearby may be skipped by the builder. | Empty method body `{}` gets no region. |
| ED-FOLD-006 | MUST | The placeholder `...` is the default; the text a builder gives is shown verbatim; placeholders are drawn with the fold-placeholder color and a rounded background (`07`). | Collapsed import list shows `...` after `import `. |
| ED-FOLD-007 | SHOULD | Folding state restored after a restart: the **fold state** of every generated region is persisted per file (by the region's range and a *signature* of the element it belongs to: names of the element and its ancestors, falling back to offsets) and restored on reopening when the file has not changed outside the IDE; unmatched entries are discarded. For quick startup the previous regions are shown first and refreshed by the first real pass ("zombie" regions), without a visible flicker for unchanged regions. | Collapse a method, close and reopen the file: still collapsed. |
| ED-FOLD-008 | MUST | The first-time default state of a region comes from the builder and the settings of section 5; later passes never re-apply defaults to regions that already have a state. Opening a file at a position inside a collapsed-by-default region (navigation) expands that region (and only those regions that intersect the navigation target). | Go to a line inside collapsed-by-default imports: the import list is expanded. |

## 4. Requirements: commands

Notation: "caret line" is the logical line of the caret; "region starting at the line" is a region
whose start offset lies on that line, the outermost one if several (the first in document order).

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-FOLD-010 | MUST | **Collapse** (per caret): if a region starts at the caret line and is expanded, collapse it; otherwise collapse the **innermost expanded region** that contains the caret offset (regions containing the offset ordered from innermost). Nothing else changes. Runs as one batch folding operation. | Caret on the line `void f() {` collapses the method body; caret inside the body of a method that has an inner expanded block collapses the inner block, a second press the method. |
| ED-FOLD-011 | MUST | **Expand** (per caret): if a region starts at the caret line and is collapsed, expand it; otherwise expand the **outermost collapsed region** that contains the caret offset (the one actually visible). | Caret on a collapsed placeholder's line expands it. |
| ED-FOLD-012 | MUST | **Expand Recursively** and **Collapse Recursively**: determine the root region as above (the region starting at the caret line; for Collapse Recursively only if it is expanded, else the first region at the offset whose state is the opposite of the goal) and set the state of that region and **all regions it contains**. | Collapse Recursively on a class collapses the class and all nested regions. |
| ED-FOLD-013 | MUST | **Collapse All** / **Expand All**: act on all regions (or, when the primary caret has a selection that overlaps regions, on the regions fully inside the selection). They use a two-step rule: step 1 toggles only regions that are not already in their "default" state class (Collapse All skips regions a builder marked "keep expanded on first Collapse All"; Expand All skips regions that the builder collapses by default); the second step runs only when the step 1 regions are all non-expandable and toggles every region of the set. In the normal case (no never-expand regions) one press does step 1 only. | In a file with imports collapsed by default, Expand All expands methods but leaves the imports (default-collapsed) as they are; a second use of Expand All does expand them. |
| ED-FOLD-014 | MUST | **Expand to level N** / **Expand all to level N** (N from 1 to 5): the root is the region starting at the caret line, else the outermost region at the caret (for the "all" variant the root is the whole file, level 1 = top-level regions). Regions at a relative depth below N are expanded, regions at depth N are collapsed, deeper regions are untouched. The depth is counted by region nesting, not by indentation. Keys are a two-step chord (Ctrl+Numpad* then the digit). | Expand all to level 1 collapses every region at depth 1 and expands nothing else. |
| ED-FOLD-015 | MUST | **Fold Selection / Remove Region** (Ctrl+.): enabled when the selection is non-empty or the caret is inside a region. With a selection: a trailing line break at the selection end is excluded; if a region with exactly that range exists, a user-defined one is **removed** and a generated one yields the hint `Cannot remove auto-generated fold region`; if the range partially overlaps existing regions a dialog `Fold Selection` / `Overlapping fold region(s) exist` with buttons `Remove` and `Cancel` (Cancel is default) asks, and Remove deletes the partially overlapping regions; then a new collapsed user-defined region with placeholder `...` is created over the range and the caret moves to the start of the range plus three characters (clamped). Without a selection: the innermost region at the caret is removed if user-defined, or toggled if generated. | Cases ED-FOLD-015. |
| ED-FOLD-016 | SHOULD | **Fold Code Block** (Ctrl+Shift+.) collapses the code block around the caret through a language handler (Java: the innermost braces block); Collapse Doc Comments and Expand Doc Comments act on all documentation comments of the file (or the selection). | Java: caret inside a block collapses that block. |
| ED-FOLD-017 | SHOULD | **Toggle Folding** flips the state of the region starting at the caret line, else the innermost region at the caret. **Custom Folding...** (Ctrl+Alt+.) lists the custom regions of the file in a popup and jumps to the chosen one. | Popup lists `Helpers`. |
| ED-FOLD-018 | MUST | All fold commands are enabled only when the editor has a project; after the operation each collapsed region containing a caret moves that caret to the region's start (not inside), and the caret and selection are otherwise unchanged. All carets are processed (Collapse and Expand per caret; All variants once). | Multi-caret Collapse collapses a region for each caret. |

## 5. Settings and defaults

Settings, Editor, General, Code Folding (`Fold by default:` group). Values are the shipped
defaults. The first group is language independent (`CodeFoldingSettings`).

| Setting (label) | Default |
|---|---|
| Imports | on (collapsed) |
| Method bodies | off |
| Documentation comments | off |
| File header | on |
| Custom folding regions | off |

Java adds (Settings, Editor, General, Code Folding, language group): one-line methods **on**
(collapse the whole simple one-statement method to `{ ... }` form), accessors **off**, inner classes
**off**, anonymous classes **off**, lambdas/closures (as `->` form) **on**, annotations **off**,
constructor/method generic parameters **on**, i18n messages **on** (replace a message lookup with its
text), suppress-warnings annotations **on**, end-of-line comment blocks **off**, multi-line
comments **off**, replace `var` with the inferred type **off**.

| Related setting | Default |
|---|---|
| Show code folding outline | on |
| Show code folding arrows / outline: `Always` vs `On mouse hover` | outline on mouse hover (registry-like default: only on hover) |
| Show bottom arrows | off (the legacy "folding endings") |
| Registry `editor.highlight.foldings` (highlight region under the caret in the gutter) | on |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-FOLD-020 | MUST | The settings of the first table decide the first-time collapsed state of imports, method bodies, doc comments, file header and custom regions; changing a setting does not change regions already created; it applies to files opened afterwards and to regions created later. | Turn `Imports` off: a newly opened file shows imports expanded. |
| ED-FOLD-021 | SHOULD | The language settings are language specific groups on the same page; unknown languages show only the base group. | Page lists `Java` group when Java support is loaded. |

## 6. Java regions (catalog)

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-FOLD-030 | MUST | **Imports**: when the file has two or more import statements, one region covers the text from the character after the first `import` keyword's following blank to the end of the last import statement; placeholder `...`; collapsed by default per setting `Imports`; removable when collapsed; none when error elements are nearby the range. A single import has no region. | Cases ED-FOLD-030. |
| ED-FOLD-031 | MUST | **File header**: a leading comment block (comments separated only by whitespace at the start of the file, ending with the first non-comment) spanning more than one line gets a region with placeholder `/.../`, collapsed per `File header` (default on); not for `package-info` documentation comments. | A license header of five lines is collapsed on open. |
| ED-FOLD-032 | MUST | **Class body** of a non-top-level class, or of any class in a file with several top-level classes: range from `{` to `}`, placeholder `{...}` (or `{}` is not created: single line shorter than the placeholder); collapsed by default per `Inner classes` for nested classes. A single top-level class gets none. Record headers and annotation arguments: placeholder `(...)`; annotations collapsed per `Annotations`. | Two top-level classes in a file: two class regions. |
| ED-FOLD-033 | MUST | **Method body**: the body block, range `{` to `}`, placeholder `{...}` (`{}` for an empty body is not created); collapsed per `Method bodies`. **One-line method** (setting on, default): when the name and the parameter list are on one line, the body has exactly one statement that fits on one line, the statement is on the line right after `{` and the closing brace on the line right after it, and the folded result fits the right margin: two grouped regions replace the text between `)` and the statement (placeholder ` { `) and between the statement and `}` (placeholder ` }`), collapsed by default, so the method shows as `int f() { return 1; }`. Otherwise the plain body region applies. | Cases ED-FOLD-033. |
| ED-FOLD-034 | MUST | **Inner blocks**: lambda bodies with a block (placeholder `{...}`), anonymous class bodies (as a closure `->` form when the setting is on and the class is a functional-interface instance: the visible text becomes the lambda form), nested blocks of statements with at least one statement (`{...}`, expanded), class initializers and enum constant bodies, anonymous and local classes. | A `for` block with statements has a `{...}` region. |
| ED-FOLD-035 | SHOULD | **Comments**: documentation comments get a region with placeholder `/**` + first non-blank text line + ` ...` + `*/` (placeholder shows the whole text when the comment has only the header line); block comments `/*...*/`; runs of adjacent line comments (separated only by whitespace) one region with placeholder `//...`; collapsed per `Documentation comments`, `End-of-line comments`, `Multi-line comments`, `File header`. Custom region comments are never part of a comment run. | Cases ED-FOLD-035. |
| ED-FOLD-036 | SHOULD | **Generic arguments**: the type argument list (text starting with `<`, ending with `>`) of a `new` expression that is the initializer of a variable whose declared type has the same arguments, and of an explicit type argument list on a method call, is replaced by the placeholder `<~>` when it is longer than 5 characters (3 for calls) and every argument resolves (not done in the quick pass); collapsed by default per `constructor/method generic parameters` (default on). With `Replace var with inferred type` on, an inferred local variable type is shown as its presentable name when it is at most 25 characters. | `List<String> l = new ArrayList<String>();` shows `new ArrayList<~>()`. |
| ED-FOLD-037 | LATER | Folding of i18n message lookups, `@SuppressWarnings`, accessor methods (simple getters and setters), module declaration blocks. | Listed so the model leaves room. |

## 7. Custom regions

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-FOLD-040 | MUST | **Comment-based regions** are defined by pairs of start and end comments, matched like parentheses by nesting inside a *root* (Java: the outermost class, and every code block; a pair can not cross roots). The recognized styles are: (1) *Visual Studio style*: a comment whose text, after leading `/`, `*`, `#` or `-` characters and blanks, starts with `region`, closed by one starting with `endregion`; the placeholder is the text after `region` (trimmed, and for `/* ... */` comments with the closing `*/` removed), or `...` when empty; (2) *NetBeans style*: a comment containing `<editor-fold` closed by one containing `</editor-fold`, placeholder from the `desc="..."` attribute (or `...`), collapsed by default when it contains `defaultstate="collapsed"`. The region range runs from the start of the start comment to the end of the end comment. Unmatched start or end comments produce nothing. | Cases ED-FOLD-040. |
| ED-FOLD-041 | MUST | Collapsed by default per `Custom folding regions` (default off), or per the `defaultstate` attribute of the NetBeans style. Max nesting depth of the syntax tree searched: 50 (registry `custom.folding.max.lookup.depth`). | Region with `defaultstate="collapsed"` is collapsed on open. |
| ED-FOLD-042 | SHOULD | `Code, Surround With, region...endregion` inserts the comment pair (surround templates, `05`). | Surround two methods. |

## 8. Interaction with caret, editing and clipboard

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-FOLD-050 | MUST | Caret movement treats a collapsed region as one unit (`ED-CARET-013`, `ED-CARET-110`): Left/Right step over it, Home/End use visual lines, Up/Down keep the horizontal position over visual lines. | Case ED-FOLD-050. |
| ED-FOLD-051 | MUST | Editing never makes the user type into hidden text: an action that would modify text inside a collapsed region (typing with the caret at its edge is fine) expands the region first, except never-expand regions which are replaced or deleted as a unit; Backspace after a never-expand region deletes the region; Delete before it likewise. | Type at the end of a collapsed region: typed text is after it. |
| ED-FOLD-052 | MUST | **Copy and Cut** use the real text of collapsed regions; selection that covers a region covers the real text. Folded state is also transferred with the clipboard to the same language file (regions folded in the source are folded in the pasted code). | Case ED-FOLD-052. |
| ED-FOLD-053 | MUST | Find, Go To, Next Error and other navigation that puts the caret or a selection inside a collapsed region expand it (all enclosing regions) unless it is never-expand. | Go to line inside a collapsed method expands it. |
| ED-FOLD-054 | MUST | Moving lines or statements that contain regions keeps their expansion state with the text (`ED-LINE-030`). Duplicate line copies the text only, new regions get default state. | Move a collapsed method down: still collapsed. |
| ED-FOLD-055 | MUST | Comment with Line Comment and similar line commands extend their range to whole lines when the range edge falls inside a collapsed region (`ED-LINE-040`). | Comment a line that ends in a collapsed region comments the whole region lines. |
| ED-FOLD-056 | SHOULD | Mouse: a click on the gutter marker toggles the region; a click on a collapsed placeholder expands it. | Click the placeholder: region expands. |
| ED-FOLD-057 | SHOULD | The vertical folding "outline" (lines from marker to marker) is drawn when `Show code folding outline` is on, and the markers (arrows) are drawn only while the mouse is over the gutter when the `On mouse hover` mode is selected (default). | Default: no arrows until hovered. |

## 9. Open questions

* Where exactly the caret goes after Collapse when it was inside the region: the source of the
  handler leaves this to the folding model (the model moves carets out of collapsed regions); case
  `ED-FOLD-010 case 2` is marked unverified.
* The `Expand all to level` numbering for forests with several roots.
* The exact blank handling of documentation comment placeholders (`ED-FOLD-035`).
* Kotlin, Python, XML and JSON builders (placeholder and default rules) are specified with each
  language; JSON folds objects to `{...}` and arrays to `[...]` (outside this file).

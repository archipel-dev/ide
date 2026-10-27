# Workbench spec 02: Project view

Status: specification (clean-room). ID prefix `WB-PV`. Priorities `MUST` / `SHOULD` / `LATER` as in `docs/specs/README.md`.
Executable cases: `cases/02-project-view.cases.md`. Tool window frame rules (anchor, Alt+1, Escape) are in `01-tool-window-framework.md`.

Scope: the `Project` tool window: panes (views), node types and their order, display names, sorting, compact / flatten / hide-empty rules,
members, scratches, file nesting, selection rules (select opened file, autoscroll), opening files, the New / Mark Directory As / delete / copy /
paste / drag actions, the context menu and speed search. Project model data (modules, roots) is defined in `platform/07-project-structure-and-scopes.md`.

Sources studied: `ic: platform/lang-impl/src/com/intellij/ide/projectView/impl/` (`ProjectViewState.kt`, `ProjectViewSharedSettings.kt`,
`ProjectViewPane.java`, `AbstractProjectViewPane.java`, `GroupByTypeComparator.java`, `FileNestingBuilder.java`, `ProjectViewFileNestingService.java`,
`nodes/ProjectViewDirectoryHelper.java`, `nodes/PsiDirectoryNode.java`, `nodes/ProjectViewProjectNode.java`), `ic: platform/lang-impl/src/com/intellij/ide/projectView/ProjectViewSettings.java`,
`ic: platform/editor-ui-api/src/com/intellij/ide/projectView/ViewSettings.java`, `ic: platform/editor-ui-api/src/com/intellij/ide/util/treeView/NodeOptions.java`,
`ic: java/java-impl/src/com/intellij/ide/projectView/impl/PackageViewPane.java`, `ic: platform/lang-impl/src/com/intellij/ide/scopeView/ScopeViewPane.java`,
`ic: platform/lang-impl/src/com/intellij/ide/scratch/ScratchTreeStructureProvider.java`, `ic: platform/lang-impl/src/com/intellij/ide/actions/CreateDirectoryOrPackageAction.java`,
`CreateDirectoryOrPackageHandler.java`, `CreateFileAction.java`, `ic: platform/platform-impl/resources/idea/LangActions.xml` (menu structure),
`ic: platform/platform-resources-en/src/messages/ActionsBundle.properties`, `ic: platform/platform-api/resources/messages/IdeBundle.properties`,
`ic: platform/platform-resources/src/keymaps/$default.xml`.

## 1. Panes (views)

The tool window id is `Project` (anchor `left`, Alt+1). Its header shows a **view selector** (dropdown) whose current entry is the title.
Panes are an extension point with an integer weight that orders them in the selector (ascending).

| Pane id | Title | Weight | Content |
|---|---|---|---|
| `ProjectPane` | Project | 0 | modules and directories as the file system under content roots (section 2) |
| `PackagesPane` | Packages | 1 | Java/Kotlin packages of all source roots, modules above them (Java area) |
| `ProjectFilesPane` | Project Files | after Packages | all files of the project directories without module grouping |
| `Scope` | the scope name (list title "Scopes") | 4 | named scopes: Scratches and Consoles, Problems, Production, Tests, Open Files, Changed Files, custom scopes |

The default pane is `Project`. The scope pane's tree for an empty scope shows the empty text "No files in the current scope." and a link
"Show all project files" that switches to the Project pane. Custom scopes are edited in Settings | Appearance & Behavior | Scopes
(settings id `project.scopes`) or through the view menu entry "Edit Scopes...".

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-PV-001 | MUST | The Project tool window has the panes `Project`, `Packages` and `Project Files` in this order and a selector listing them; the selected pane persists per project. | Select Packages, reopen the project: Packages is selected. |
| WB-PV-002 | MUST | `Project` is the pane shown for a new project. | Open a fresh project: title "Project". |
| WB-PV-003 | SHOULD | The scope pane offers the predefined scopes "Scratches and Consoles", "Problems", "Production", "Tests", "Open Files" (Open Files is scope.open.files) and any custom scope; its empty state is as above. | Choose scope "Tests" in a project without tests: empty text "No files in the current scope." |
| WB-PV-004 | MUST | The Project pane applies the *same* view options (section 3) and the *same* node ordering as the other file-based panes, but options are stored per pane id (each pane remembers its own flatten / compact / sort values). | Turn "Flatten Packages" on in Packages: Project pane's value is unchanged. |

## 2. Nodes of the Project pane and their order

Root children, in this order:

1. one node per **top-level content root** (a module content root that is not inside another content root). When a module has exactly one top-level
   root, the node is that directory (a **directory node** carrying the module information); when it has several, a **module node** that contains them.
   Roots that belong to no module are plain directory (or file) nodes. With "Show Modules" off module nodes are not used (see 3). Unloaded modules are shown the same way.
2. **External Libraries**, only if "Show Libraries Contents" is on (default on): a node holding SDKs and library entries.
3. **Scratches and Consoles**, only if "Show Scratches and Consoles" is on (default on): holds scratch files and consoles (see `08-file-operations-and-scratches.md`).

Directory node display (sources: `PsiDirectoryNode`):

* text is the directory name; a directory that is a module content root, whose parent is not a module node, shows `name` plus, when the module name
  differs from the directory name (case-insensitive compare), the module name in bold in square brackets: `demo [demo-app]`. When the names match, the directory name alone is shown in bold.
* location text (grey, after the name): for module/project roots a comma separated list of **root type** (`library root`, `sources root`,
  `test sources root`, `resources root`, `test resources root`, optionally prefixed `generated `) and the **path**, path shortened to `~` for the user home; the path is shown only when "Show URL" holds and the parent is the project or a module (default: shown).
* module roots, source roots and excluded folders have distinct icons; a module content root is initially expanded.

File node display: file name, file-type icon; with "Show Members" on, supported files expand into classes and members (Java area); with
"Show Visibility Icons" on, a visibility icon precedes the member name.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-PV-010 | MUST | The root children are ordered: content-root/module nodes, then External Libraries, then Scratches and Consoles; the last two only when their options are on. | Project with one module, defaults: root has 3 children in the order `demo`, `External Libraries`, `Scratches and Consoles`. |
| WB-PV-011 | MUST | A module whose single content root is a directory is displayed as that directory node (no extra module level); a module with several roots gets a module node with its roots as children. | Module `app` with roots `app` and `shared`: node `app` (module) with two children. Module with one root: one directory node. |
| WB-PV-012 | MUST | Display text of a module-root directory: `name [module]` when names differ (module part bold), `name` bold when they match, plus the grey location string of the rules above. | Directory `demo` in module `demo-app`: `demo [demo-app]  ~/IdeaProjects/demo`. Directory `demo` in module `demo`: bold `demo`, grey path. |
| WB-PV-013 | MUST | A directory that lies under a content root is shown only when it is in project content, or excluded while "Show Excluded Files" is on (default on). With the option off, excluded directories and the project/workspace metadata files are hidden. | Excluded folder `target`: visible with option on (excluded icon), hidden with it off. |
| WB-PV-014 | SHOULD | The project configuration directory (`.idea` for IntelliJ-format projects) is treated as project metadata. Whether it is listed is decided by the helper that decides visibility of project/workspace files (see open question 1). | See open question 1. |
| WB-PV-015 | MUST | Directories that are ancestors of content roots but not themselves content (for example a project root with modules in subfolders) are shown as plain directory nodes leading to the roots; non-content siblings are not listed. | Project root `repo` not a content root, modules `repo/a`, `repo/b`: tree `repo` > `a`, `b`; other folders of `repo` are absent. |
| WB-PV-016 | MUST | Children of a directory node are the directories and files that are in project content (or excluded when shown), then ordered by section 4. | Directory with `src`, `README.md`, `target` (excluded): `src`, `target`, `README.md` in default sort. |
| WB-PV-017 | MUST | The tool window has no content (but does not fail) when the project has no content roots. | Empty project: tree is empty, no panic. |

## 3. View options (gear menu) and defaults

The **gear menu** of the tool window (the "Options" button, `ProjectView.ToolWindow.*Actions` groups) contains three groups plus entries. Texts are the
reference texts. Defaults are what a user without saved state observes (shared-settings defaults).

| Group | Entry (text) | Action id | Default | Effect |
|---|---|---|---|---|
| Behavior | Always Select Opened File | `ProjectView.AutoscrollFromSource` | off | selecting an editor tab selects its file in the tree |
| Behavior | Open Files with Single Click | `ProjectView.AutoscrollToSource` | off | selecting a file node opens it in the editor |
| Behavior | Open Directories with Single Click | `ProjectView.OpenDirectoriesWithSingleClick` | off | single click expands/collapses a directory |
| Behavior | Enable Preview Tab | `ProjectView.OpenInPreviewTab` | off | opening by selection uses the preview tab |
| Appearance | Show Members | `ProjectView.ShowMembers` | off | files expand to classes and members |
| Appearance | Show Visibility Icons | `ProjectView.ShowVisibilityIcons` | off | visibility icon before member name |
| Appearance | Compact Directories | `ProjectView.CompactDirectories` | off | merge a directory with its single sub-directory into one node |
| Appearance | Hide Empty Middle Packages | `ProjectView.HideEmptyMiddlePackages` | on | skip packages that contain only one sub-package |
| Appearance | Flatten Packages | `ProjectView.FlattenPackages` | off | show all packages of a source root as one flat list |
| Appearance | Abbreviate Qualified Names | `ProjectView.AbbreviatePackageNames` | off | show qualified package names in short form |
| Appearance | Show Modules | `ProjectView.ShowModules` | on | show module names for corresponding roots |
| Appearance | Flatten Modules | `ProjectView.FlattenModules` | off | show modules in a flat list |
| Appearance | Show Excluded Files | `ProjectView.ShowExcludedFiles` | on | show excluded folders and files |
| Appearance | Show Libraries Contents | `ProjectView.ShowLibraryContents` | on | show External Libraries node |
| Appearance | Show Scratches and Consoles | `ProjectView.ShowScratchesAndConsoles` | on | show that root node |
| Appearance | File Nesting... | `ProjectView.FileNesting` | rules on | opens the nesting dialog (section 6) |
| Appearance | Customize Tree Views... | `ProjectView.CustomizeTrees` | - | choose which panes exist (LATER) |
| Sort | Folders Always on Top | `ProjectView.FoldersAlwaysOnTop` | on | directories before files |
| Sort | Sort by Name | `ProjectView.SortByName` | on (default sort key) | by name |
| Sort | Sort by Type | `ProjectView.SortByType` | off | by file type, then name |
| Sort | Sort by Modification Time (Newest First) / (Oldest First) | `ProjectView.SortByTimeDescending` / `SortByTimeAscending` | off | by last-modified time |
| Sort | Manual Order | `ProjectView.ManualOrder` | off | user-defined order via drag and drop (LATER) |

Other actions in the header: **Select Opened File** (crosshair button, selects the current editor file), **Expand All** (`ProjectViewExpandAll`, text
"Expand All"; default keymap `ExpandAll` = Ctrl+NumPad+ / Ctrl+=), **Collapse All** (Ctrl+NumPad- / Ctrl+-), **Edit Scopes...** and **Hide** (hide the tool window).
The `Sort` entries are mutually exclusive (radio behavior) except "Folders Always on Top" and "Manual Order", which are independent toggles.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-PV-020 | MUST | The gear menu contains the groups and entries of the table, with these texts and in this order (Behavior, Appearance, Sort). Unsupported entries are disabled, never missing. | The menu dump equals the table's texts. |
| WB-PV-021 | MUST | Defaults equal the "Default" column. | A fresh profile reports each option with that value. |
| WB-PV-022 | MUST | Options persist per project (workspace) and per pane, and take effect immediately without losing the selection or the expanded paths when the nodes still exist. | Toggle "Show Members", expand a file, toggle again: selection and expanded directories are preserved. |
| WB-PV-023 | MUST | The sort entries are exclusive: choosing one unchecks the others. | Choose Sort by Type, then by Modification Time (Newest First): only the latter is checked. |
| WB-PV-024 | SHOULD | Settings default for "Open Files with Single Click" (autoscroll to source) comes from the global default `defaultAutoScrollToSource` (false). | Same as the table. |

## 4. Sorting

Comparison of two siblings, applied in this order; the first non-equal result wins:

1. **Sort order class**: when "Folders Always on Top" is on, directories (`FOLDER` class) sort before everything else. (Attached projects: the primary module is first.)
2. If "Manual Order" is on, the explicit manual order key.
3. **Type weight** when folders on top: directories have a nonzero type weight and come first; files and other nodes weight 0. Directories of different type weights keep weight order.
4. By the chosen sort key:
   * `Name`: the node's sort key (its name), compared **naturally, case-insensitively** (`a2` before `a10`; `Readme` and `readme` adjacent);
   * `Type`: type sort key (file type name), then name;
   * `Modification Time`: ascending = oldest first, descending = newest first; ties by name.
5. When "Abbreviate Qualified Names" is on: qualified-name sort key, natural compare.
6. Fallback: alphabetical compare of the display text.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-PV-030 | MUST | Siblings are ordered by the algorithm above; with defaults: directories first, then files, each natural-case-insensitive by name. | Children `b.txt`, `Alpha.txt`, `lib`, `a10.txt`, `a2.txt`, `Docs` display as `Docs`, `lib`, `a2.txt`, `a10.txt`, `Alpha.txt`, `b.txt`. |
| WB-PV-031 | MUST | With "Folders Always on Top" off, directories and files are interleaved by name. | Same children, option off: `a2.txt`, `a10.txt`, `Alpha.txt`, `b.txt`, `Docs`, `lib`. |
| WB-PV-032 | MUST | Sort by Type groups files of the same file type together (types ordered by type name), then by name inside a group. | `a.java`, `b.txt`, `c.java` sorted by type: `a.java`, `c.java`, `b.txt` (file type display names compared alphabetically: "Java" before "Plain text"; see open question 3). |
| WB-PV-033 | SHOULD | Sort by modification time uses the file's last-modified time; directories keep their own order when folders are on top. | Files modified at T1<T2<T3: Newest First lists T3, T2, T1. |
| WB-PV-034 | LATER | Manual Order: drag a node to reorder siblings; order persisted per parent. | n/a |

## 5. Compact directories, empty middle packages, flatten

Definitions (for directories under a source root of a language with packages, and for plain directories where noted):

* **Hide Empty Middle Packages** (default on): a directory that contains **no files in project content, only exactly one sub-directory** (a *middle*
  directory) is not shown as its own node when it is not a source root; its child is shown in its place with the qualified name `a.b` in the name:
  `com`, `com/acme`, `com/acme/app` with files only in `app`: one node `com.acme.app` below the source root. Applies only where the helper supports it (package roots); plain folders do not compact unless "Compact Directories" is on.
* **Compact Directories** (default off): any directory that contains only one directory (and no files) is merged with it into one node with names joined by `.`: the display text is the chain of names joined with ".".
* **Flatten Packages** (default off): under a source root, show every non-empty package as a direct child, named by its full qualified name, instead of a hierarchy. Subdirectories that are not packages keep their hierarchy.
* **Abbreviate Qualified Names** (default off): show the leading segments of a qualified package name by their first letters (`c.a.app`), full name as tooltip.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-PV-040 | MUST | With Hide Empty Middle Packages on, a chain of middle directories collapses into the node of the first directory that has files or several sub-directories; the node text is the dotted chain; its location is the leaf directory. A source root itself is never collapsed away. | `src/com/acme/app/Main.java`: under `src` a single node `com.acme.app` containing `Main.java`. |
| WB-PV-041 | MUST | With it off, each directory is its own node. | Same data: `src` > `com` > `acme` > `app` > `Main.java`. |
| WB-PV-042 | MUST | A middle directory that has files is **not** collapsed. | `com/acme/Util.java` plus `com/acme/app/Main.java`: nodes `com.acme` (with `Util.java`) > `app`. |
| WB-PV-043 | MUST | Compact Directories merges single-child directory chains regardless of source roots: text `a.b.c`. Expanding/collapsing acts on the merged node; selecting a file below it selects the file, not the merged directory. | `docs/guide/img` with only images in `img`: tree `docs/guide/img/a.png` with Compact Directories on shows one node `docs.guide.img` containing `a.png`. |
| WB-PV-044 | MUST | Flatten Packages lists each package that contains files as a direct child of the source root, sorted by qualified name; packages that only contain packages are omitted. | `com.acme` (files), `com.acme.app` (files), `com` (empty): children `com.acme`, `com.acme.app`. |
| WB-PV-045 | SHOULD | Abbreviate Qualified Names shortens every segment but the last of dotted names to one letter. | `com.acme.app` shows `c.a.app`; tooltip: full name. |
| WB-PV-046 | MUST | Selecting a file via "Select Opened File" while the target directory is collapsed into a compact node selects the file under that node and expands it. | Open `Main.java`, press Select Opened File with Compact on: the `com.acme.app` node expands, `Main.java` selected. |

## 6. File nesting

"File Nesting..." opens a dialog that edits a list of **rules** `parent suffix -> child suffix` and the master switch "use file nesting rules"
(default **on**). Rules persist in the global UI settings. Default rules are contributed by extensions (languages add their own, for example a
generated `foo.js` and `foo.js.map` below `foo.ts`).

Algorithm (rules are matched on **file names**, case-insensitively, siblings in one directory only):

1. Normalize the rule set: ignore rules with an empty suffix or equal parent and child suffixes. Take the **transitive closure**: if `A -> B` and `B -> C` then `A -> C`.
2. For each file name and each rule, the file matches as *parent* if it ends with the parent suffix, as *child* if it ends with the child suffix. If it matches both, the longer suffix decides (a longer parent suffix wins parent; otherwise it is the child).
3. A rule produces an edge `P -> C` when a file `base + parentSuffix` and a file `base + childSuffix` both exist in the directory.
4. A child that is itself a target of any edge loses its outbound edges (so a 3-cycle collapses and in `A -> B -> C` only `A -> B` stays among the closure edges after removal of `B -> C`; the closure edge `A -> C` keeps `C` under `A`).
5. In the tree the child nodes are shown **as children of the parent file node**, which becomes expandable. A child can appear under more than one parent.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-PV-050 | SHOULD | With nesting on and rule `.ts -> .js`, `foo.ts` and `foo.js` in a directory render `foo.js` as a child of `foo.ts`. | Case WB-PV-050 case 1. |
| WB-PV-051 | SHOULD | With the master switch off no files are nested; rules stay stored. | Switch off: both files are siblings. |
| WB-PV-052 | SHOULD | A nested child is not listed again at the directory level. | In the case above the directory has one entry `foo.ts`. |
| WB-PV-053 | SHOULD | Rule matching is case-insensitive; the child must have the same base name. | `FOO.TS` and `foo.js`: bases differ in case only: nested (case-insensitive compare). `bar.js` does not nest under `foo.ts`. |
| WB-PV-054 | SHOULD | The dialog rejects an empty suffix and equal parent and child suffixes. | Adding `.ts -> .ts` is refused. |
| WB-PV-055 | LATER | The rule editor UI (add, remove, reset to defaults). | n/a |

## 7. Selection and opening files

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-PV-060 | MUST | **Select Opened File** (header crosshair, also "Select File in Project View" / "Select Directory in Project View" from Select In, Alt+F1): selects and scrolls to the node of the current editor file, expanding collapsed ancestors, switching to a pane that can show the file if the current one cannot (order: current pane first). If the file is not in the current pane's content, a pane that contains it is selected. | Open `src/a/B.java` deep in a collapsed tree, press the button: all ancestors expanded, `B.java` selected and scrolled into view. |
| WB-PV-061 | MUST | "Always Select Opened File" (default off): every time the selected editor tab changes the tree selection follows, without stealing focus from the editor. | Option on: click tab `A.java`: `A.java` selected; caret stays in editor. |
| WB-PV-062 | MUST | Enter, double-click and F4 (`EditSource`, text "Edit Source") open the selected file in the editor and give it focus; Ctrl+Enter (`ViewSource`) opens it without moving focus; Shift+F4 opens in a new window; Shift+Enter opens in the right split. On a directory Enter/double-click expands or collapses. | Select `A.java`, Enter: editor shows it with focus. Ctrl+Enter: opens it, focus stays in tree. |
| WB-PV-063 | MUST | "Open Files with Single Click" (autoscroll to source, default off): selecting a file (mouse, arrows) opens it without focus change; with "Enable Preview Tab" it opens in the preview tab. | Option on: arrow down onto `A.java`: it opens; focus stays in tree. |
| WB-PV-064 | MUST | "Open Directories with Single Click" (default off): a single click on a directory toggles it; otherwise double-click or Enter. | Option on: click `src`: expanded. |
| WB-PV-065 | MUST | Arrow keys: Right expands a collapsed node, otherwise moves to its first child; Left collapses an expanded node, otherwise moves to the parent; `*` (NumPad multiply) expands the selected node recursively; Up/Down move selection; Home/End first/last. | Select collapsed `src`, Right: expanded, selection on `src`; Right again: first child. |
| WB-PV-066 | MUST | **Expand All** (Ctrl+NumPad+ or Ctrl+=) expands every node; **Collapse All** (Ctrl+NumPad- or Ctrl+-) collapses all and keeps the root's children visible. | After Collapse All only the root's direct children are visible. |
| WB-PV-067 | MUST | Initial state: nodes flagged initially-expanded (module content roots) expand when first shown; the previous expanded paths and the selection are restored on project open. | Close and reopen: expanded paths and selection as before. |

## 8. Speed search

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-PV-070 | MUST | Typing printable characters while the tree has focus (no popup, no edit) opens **speed search**: a small field showing the typed text; the tree selects the first matching visible node and highlights the matched part in every visible node. | Visible nodes in order `server`, `service`, `util`; typing `ser` selects `server`, highlights `ser` in `server` and `service`, not in `util`. |
| WB-PV-071 | MUST | Matching is case-insensitive, matches **word-start and camel-hump** abbreviations (`sm` matches `StringMap`) and substrings; only nodes in already-visible (expanded) parts are searched, plus nodes the tree can load lazily on screen. | Type `sm`: `StringMap.java` matches. |
| WB-PV-072 | MUST | Up and Down move to the previous or next match; Enter activates the selected node (opens file) and closes the search; Backspace removes a character; Escape closes the search and restores the previous selection state (selection stays on the last match). | Type `ser`, Down: next match; Escape: field closes. |
| WB-PV-073 | SHOULD | If nothing matches the field turns red-ish (error state) and the selection is unchanged. | Type `zzz`: no selection change, field in error state. |

## 9. New, Mark Directory As, delete, copy, paste, drag

### 9.1 Context menu

`ProjectViewPopupMenu`, top to bottom (separators marked `---`): **New** submenu (`WeighingNewGroup`), Associate with File Type, `---`, Cut / Copy /
Copy Paths... / Paste group, Edit Source (text "Edit Source"), `---`, Find Usages, Find in Files, Replace in Files (Inspect Code is inserted after Replace in Files), `---`, Rename, Refactor submenu,
`---`, Reformat Code, Optimize Imports, Delete..., Mark File as (type override), `---`, Open In / reveal group (Show in File Manager ...), `---`, Version control group, Reload from Disk (`SynchronizeCurrentFile`, text "Reload from Disk"),
`---`, Go to link target, `---`, Compare group (Compare Files, Compare File with Editor), `---`, External tools group, `---`, **Mark Directory As** (`MarkRootGroup`).

### 9.2 New

`NewElement` ("New..." in the menu bar File | New; in the Project view "New File or Directory..." variant, shortcut **Alt+Insert**) and `NewElementSamePlace`
("New in This Directory...", Ctrl+Alt+Insert) open the **New** popup: a list whose first entries are language and file-template actions weighted for the
selected directory (for a Java source root: Java Class, Kotlin Class/File, ...), then **File**, **Scratch File** (Ctrl+Alt+Shift+Insert), **Directory** (or **Package** inside a package root), then templates.

* **File** (`NewFile`): a popup titled "New File" with one text field; Enter creates; the field validates live. The name may contain `/` (and `\` on Windows) to create intermediate directories; segments `.` are ignored, `..` goes to the parent directory, a leading `~` is the user home.
* **Directory / Package** (`NewDir`, menu text **Directory** or **Package** depending on whether the target is a package root): popup titled "New Directory" with prompt "Enter new directory name:", or "New Package" with prompt "Enter new package name:". Package names use `.`; directory names use `/` (and `\`) as delimiters; the popup lists suggested directories (for example `src/main/java`, `src/test/resources`) to create several at once.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-PV-080 | MUST | Alt+Insert in the tree opens the New popup for the selected directory (or the parent directory of a selected file). In the editor it opens the context-appropriate New popup. | Select `src`, Alt+Insert: popup lists `File`, `Scratch File`, `Directory` (or `Package`) among its entries. |
| WB-PV-081 | MUST | **New File**: Enter with a valid name creates the file, opens it in an editor, selects it in the tree. A name containing `/` creates the directories on the way. An empty name does nothing (popup stays) and shows no error until Enter; name `a/b/c.txt` creates `a/b` and `c.txt`. | See cases WB-PV-081 case 1 and 2. |
| WB-PV-082 | MUST | **New File** errors (shown inline under the field, popup stays open): existing file: `A file with the name 'x' already exists`; existing directory with that name: `A directory with the name 'x' already exists`; invalid characters: the platform file-name error ("Incorrect name" fallback). | Create `README.md` twice: second time error `A file with the name 'README.md' already exists`. |
| WB-PV-083 | MUST | **New Directory**: `a/b/c` creates the whole chain; existing intermediate directories are reused; the last existing directory is an error `A directory with the name 'c' already exists`; an intermediate existing **file** is an error `A file with the name 'a' already exists`. A final segment `.` or `..` is an error `'..' is an invalid directory name`. A leading `~` refers to the user home. `..` segments move up one directory. | See cases. |
| WB-PV-084 | MUST | **New Package**: delimiters `.`; each segment must be a valid package name else the warning `Invalid package name; it's impossible to create a Java class inside`; creating `com.acme` in an empty source root creates `com/acme`. | New Package `com.acme` in `src`: directories `src/com/acme`; node shown as `com.acme` with Hide Empty Middle Packages on. |
| WB-PV-085 | SHOULD | Warnings (non-blocking, shown in the popup): the name is an ignored name: `Trying to create a directory with an ignored name ('x'); the result will not be visible` (package variant: `... a package ...`); a directory name with a dot but no path delimiter: `Note: "." in the name is treated as a regular character. Use "/" instead if you mean to create nested directories`. | New Directory `a.b`: warning text above, Enter still creates `a.b`. |
| WB-PV-086 | SHOULD | Creating a directory whose name looks like a file name of a known type (exactly one dot, known extension) asks "The name you entered looks like a file name. Do you want to create a file named x instead?" with buttons `Yes, Create File`, `No, Create Directory` (`No, Create Packages` for packages), `Cancel`; title "File Name Detected". Controlled by the registry flag `ide.suggest.file.when.creating.filename.like.directory` (default value not read, open question 7; Archipel: prompt enabled). | Prompt enabled, New Directory `a.txt`: the dialog appears; `No, Create Directory` creates directory `a.txt`; `Yes, Create File` creates file `a.txt`; `Cancel` creates nothing. |
| WB-PV-087 | MUST | New items are created in one undoable command ("Create File", "Create Directory", "Create Package") and recorded in local history. | Ctrl+Z after creation removes it (Edit > Undo shows the command name). |

### 9.3 Mark Directory As

Submenu "Mark Directory As" (group `MarkRootGroup`), shown for directories in a module: **Sources Root**, **Tests Root**... (the reference lists the module
source root types of the installed languages: Sources, Tests, Resources, Test Resources), **Excluded**, **Generated Sources Root**, and the reverse entries **Not Excluded**, **Unmark** (text "Unmark"; for several selected directories "Unmark"; for one root "Unmark as {type}"; for excluded: "Cancel Exclusion").
Group title in the content-root variant: "Mark Directory As Content Root".

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-PV-090 | MUST | "Excluded" marks the directory as excluded from the module; it keeps showing (excluded icon) when "Show Excluded Files" is on. "Not Excluded" cancels it. | Mark `build` Excluded: icon changes; search scopes no longer contain it. |
| WB-PV-091 | MUST | Marking Sources / Tests / Resources / Test Resources sets the root type of that directory; the directory gets the matching icon and location text (`sources root` etc.). Entries are shown only when applicable (not for a directory already of that type; a directory cannot be both). | Mark `src` as Sources Root: location text `sources root`. |
| WB-PV-092 | MUST | The change is written to the module's persisted definition and the project model updates before the next tree refresh. | Reload: marking is preserved. |
| WB-PV-093 | SHOULD | "Unmark" removes the special type; "Cancel Exclusion" for an excluded directory. | Unmark `src`: icon back to plain folder. |

### 9.4 Delete, copy, paste, drag and drop, rename, move

| Action | Default shortcut | Behavior |
|---|---|---|
| Delete (`$Delete`) | Delete (macOS also Backspace / Cmd+Backspace) | prompt dialog "Delete" listing what will be deleted, buttons `Delete` / `Cancel`; code elements offer "Safe delete (with usage search)" |
| Copy (`$Copy`) | Ctrl+C, Ctrl+Insert | copies the selection (files as a file list, plus text paths) |
| Cut (`$Cut`) | Ctrl+X | marks for move |
| Paste (`$Paste`) | Ctrl+V | pastes files into the selected directory: copy (or move after Cut) |
| Copy Paths (`CopyPaths`) | Ctrl+Shift+C | copies the absolute path(s) of the selection (menu: "Copy Paths") |
| Copy Reference (`CopyReference`) | Ctrl+Alt+Shift+C | copies a reference (relative path or qualified name) |
| Copy... (`CopyElement`) | F5 | dialog "Copy" with target name and directory |
| Move... (`MoveElement`) | F6 | dialog to choose the target directory |
| Rename (`RenameElement`) | Shift+F6 | rename dialog / inline edit; also F2 on a file node is not bound by default |
| Reload from Disk (`SynchronizeCurrentFile`) | none | re-reads the selected files and directories from disk |
| Show File Path (`ShowFilePath`) | Ctrl+Alt+F12 | popup of the path hierarchy |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-PV-100 | MUST | Delete on one or more files/directories shows a confirmation, then removes them (to the trash when "Move files to the bin instead of deleting permanently" is on, otherwise permanently), updates the tree and closes their editors. The confirmation for directories mentions that all files and subdirectories will be deleted. | Select `old/`, Delete: dialog "Delete"; confirm: directory gone, tabs of its files closed. |
| WB-PV-101 | MUST | Ctrl+C then Ctrl+V on a selected directory copies the file(s) into it; on a name clash the Copy dialog opens with a unique suggested name (format: open question 4) and the user can change it. Cut then Paste moves. | Copy `a.txt`, select `docs`, paste: `docs/a.txt` exists. |
| WB-PV-102 | MUST | Drag and drop of a node onto a directory node moves it (confirmation via the Move dialog for code elements); with Ctrl held it copies; a drop onto a file node uses that file's parent directory; dropping a directory onto itself or a descendant is rejected. | Drag `b.txt` onto `docs`: `docs/b.txt`. Drag `docs` onto `docs/img`: not allowed (no drop cursor). |
| WB-PV-103 | MUST | Rename via Shift+F6 on a file renames the file; on a directory renames it (and for a package renames the package, Java area); invalid or clashing names are rejected with the platform message. | Rename `a.txt` to `b.txt` when `b.txt` exists: error shown, nothing changes. |
| WB-PV-104 | SHOULD | Copy Paths puts the absolute path(s), one per line, on the clipboard; Copy Reference puts the path relative to the project root for files. | Select `src/a.txt`: clipboard `/abs/proj/src/a.txt`. |
| WB-PV-105 | MUST | All file operations in the view are undoable as one step each and respect read-only status (files that are read-only prompt to make them writable or fail with `... contains read-only file(s). Delete it anyway?`). | See WB-PV-100. |
| WB-PV-106 | SHOULD | Reload from Disk re-syncs only the selected files (see `platform/01-vfs-and-documents.md` PLAT-VFS-102). | External edit then Reload: tree and editor show the new content. |

## 10. Inventory coverage (`inventory/project-view.md`)

| Inventory item | Where | Status |
|---|---|---|
| ProjectView.* options (22 actions) | section 3 | covered |
| NewElement, NewElementSamePlace, NewFile, NewDir | section 9.2 | covered |
| MarkAsContentRoot, MarkExcludeRoot, UnmarkRoot, MarkGeneratedSourceRoot, UnmarkGeneratedSourceRoot | section 9.3 | covered |
| ProjectViewEditSource, ProjectViewExpandAll, ShowFilePath, ReverteOverrideFileTypeAction | sections 7, 9 | covered (override file type: LATER detail) |
| NewModule, ImportModule, ImportModuleFromImlFile, NewModuleInGroup, ModuleSettings, ShowProjectStructureSettings (Ctrl+Alt+Shift+S), NewModuleInfo, NewProject | project model / welcome (`06-welcome-new-project-open.md`) | LATER: project structure dialog is not part of the workbench release; New Project covered in 06 |
| Tool window `Project` | section 1, 01 | covered |
| Settings page `lombok` (Lombok plugin) | plugin | LATER: plugin, not in the kit's workbench |
| `inventory/ui-widgets.md` (91 generic list / tree / table / popup / scroll-pane key actions such as `List-selectNextRow`, `FullyExpandTreeNode`, `ClickLink`, `CollapsiblePanel-toggle`, `CloseGotItTooltip`) | the arrow, Home/End, PageUp/PageDown, NumPad-multiply behavior of trees in WB-PV-065 is the tree subset | LATER here: the generic component key map belongs to the UI kit (`docs/design-rules.md`, component contracts), not to the workbench |
| `inventory/tasks-contexts.md` (12 actions: `tasks.switch` Alt+Shift+T, `tasks.goto` Alt+Shift+N, `context.save` Alt+Shift+S, `context.load` Alt+Shift+L, `context.clear` Alt+Shift+X, `tasks.close` Alt+Shift+W, `tasks.open.in.browser` Alt+Shift+B, ...) and the settings page `tasks` | the Tasks and Contexts plugin | LATER: issue-tracker integration plugin, not part of the first workbench release (page `tasks` is listed as LATER in `05-settings-dialog-and-keymap.md`) |

## 11. Open questions

1. `.idea`: `ProjectViewDirectoryHelper` exposes a hook that hides the configuration directory by default but the reference tree lists it in practice (as an excluded-looking folder in the Project Files pane only when "Show Excluded Files" holds). Needs a reference run to fix: default visibility of `.idea` in the Project pane.
2. Default file-nesting rules: contributed by language plugins not present in the sparse source tree; Archipel baseline list is open (propose TypeScript/JS only).
3. Exact **type sort order** for "Sort by Type" (file type names compared case-insensitively?).
4. Name suggested by Paste on a clash (the Copy dialog suggests `<name>` with a numeric suffix); exact format needs a reference run.
5. "Project Files" and the scope panes' exact titles and weights besides `Scope` = 4, `PackagesPane` = 1, `ProjectPane` = 0 were not in the sparse source.
6. Whether the Project view shows a Safe-mode/trust banner (`untrusted.project.notification.*`) is covered in `06-welcome-new-project-open.md`.
7. Default of the registry flag `ide.suggest.file.when.creating.filename.like.directory` (WB-PV-086).

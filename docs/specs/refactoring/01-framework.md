# 01. Refactoring framework: menu, handlers, conflicts, preview, undo, in-place mode

Status: specification, framework level. Requirement prefix `REF-FRM`. Language-specific behavior
(Java, Kotlin) of each refactoring is in `02-rename.md` to `07-safe-delete-and-others.md`; this
file defines what all of them share.
Priorities: `MUST` / `SHOULD` / `LATER`. Executable cases: `cases/01-framework.cases.md`.

ic: `platform/lang-impl/src/com/intellij/refactoring/actions/{BaseRefactoringAction,RefactoringQuickListPopupAction}.java`,
`platform/lang-impl/src/com/intellij/refactoring/{BaseRefactoringProcessorUi,RefactoringUiServiceImpl}.java`,
`platform/refactoring/src/com/intellij/refactoring/{BaseRefactoringProcessor,ConflictsDialogBase,RefactoringHelper,RefactoringSettings}.java`,
`platform/refactoring/src/com/intellij/refactoring/util/CommonRefactoringUtil.java`,
`platform/lang-impl/src/com/intellij/refactoring/ui/ConflictsDialog.java`,
`platform/lang-impl/src/com/intellij/refactoring/rename/inplace/{InplaceRefactoring,TemplateInlayUtil,MemberInplaceRenamer}.*`,
`platform/lang-impl/src/com/intellij/refactoring/introduce/inplace/OccurrencesChooser.java`,
`platform/platform-impl/resources/idea/LangActions.xml` (menu groups),
`platform/platform-resources/src/keymaps/$default.xml`,
`platform/refactoring/resources/messages/RefactoringBundle.properties`,
`java/openapi/resources/messages/JavaRefactoringBundle.properties`,
`platform/ide-core/resources/messages/ApplicationBundle.properties`,
`java/java-impl-refactorings/src/com/intellij/refactoring/OptimizeImportsRefactoringHelper.java`.

## 1. The Refactor menu and the registered actions

The main menu has a **Refactor** menu directly after **Code**. Its content, in this order
(separators shown as `---`; `[sub]` is a submenu):

```
Refactor This...                                   Ctrl+Alt+Shift+T
Rename...                                          Shift+F6
Rename File...            (only where a file can be renamed in the current context)
Change Signature...                                Ctrl+F6
---
[sub] Extract/Introduce
      Introduce Variable...                        Ctrl+Alt+V
      Introduce Constant...                        Ctrl+Alt+C
      Introduce Field...                           Ctrl+Alt+F
      Introduce Parameter...                       Ctrl+Alt+P
      ---
      Introduce Functional Parameter...            Ctrl+Shift+Alt+P     (Java)
      Introduce Functional Variable...                                  (Java)
      Introduce Parameter Object...
      ---
      Extract Method...                            Ctrl+Alt+M
      Replace Method With Method Object...                              (Java)
      ---
      Extract Delegate...
      Extract Interface...
      Extract Superclass...
Inline...                                          Ctrl+Alt+N
Find and Replace Code Duplicates...                                     (Java, after Inline)
---
Move...                                            F6
Copy...                                            F5
Safe Delete...                                     Alt+Delete
---
Pull Members Up...
Push Members Down...
---                                                                      (Java block "RefactoringMenu4")
Type Migration...                                  Ctrl+Shift+F6
Make Static...
Convert To Instance Method...
---
Use Interface Where Possible...
Replace Inheritance with Delegation...
---
Encapsulate Fields...
[sub] Migrate Packages and Classes
---
Invert Boolean...
```

In Kotlin the Extract/Introduce submenu shows `Extract Function...` (same shortcut as Extract
Method) in place of `Extract Method...`, `Extract Function to Scope...` (`Ctrl+Alt+Shift+M`),
`Introduce Property...` (same shortcut as Introduce Field) and `Introduce Type Alias...`
(`Ctrl+Alt+Shift+A`); in the editor's floating code toolbar the "Extract" group offers Extract
Function/Method (first), Introduce Variable, Introduce Constant and Introduce Parameter.

Shortcuts are the default keymap ones (`$default`): `Shift+F6`, `F6`, `F5`, `Ctrl+F6`,
`Ctrl+Shift+F6`, `Alt+Delete`, `Ctrl+Alt+V/C/F/P/M/N`, `Ctrl+Alt+Shift+T`.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| REF-FRM-001 | MUST | The Refactor menu has the structure above, in this order, with the exact action texts. Items whose refactoring does not exist for the current language are hidden, items that exist but do not apply to the current caret or selection are disabled (greyed) and keep their position. The "Rename File..." item is shown only when the context is a file or a class whose rename would also rename a file (see REF-REN-051). | Open the menu in a Java file with the caret on a local variable: Rename, Change Signature (disabled), Introduce Variable, Inline are enabled; Pull Members Up is disabled. In a Kotlin file the Introduce submenu lists `Extract Function...`. |
| REF-FRM-002 | MUST | All default shortcuts listed above are bound. The same actions are reachable from the editor context menu (Refactor submenu), the Project view and Structure view context menus (the refactorings that work on elements without an editor: Rename, Move, Copy, Safe Delete, Change Signature, Pull Up, Push Down, Extract Interface/Superclass, Make Static, ...) and from Search Everywhere (Actions tab). | Press each shortcut in an editor with a suitable caret; the matching dialog or in-place mode starts. |
| REF-FRM-003 | MUST | Action availability rule: an action is hidden when there is no project, when the file is a compiled/library file (default `disableOnCompiledElement = true`; Rename and Safe Delete on library elements are enabled and fail later with REF-FRM-012), or when no handler exists for the language of the file or element. It is disabled (visible but greyed) when a handler exists but the element at the caret does not qualify. In the editor the target is the element at the caret (the element a reference at the caret resolves to takes priority over the surrounding declaration); without an editor the target is the selected tree nodes. | Caret on a keyword in Java: Rename is disabled, Introduce Variable is enabled only if an expression is found. In a decompiled `.class` file Extract Method is hidden. |
| REF-FRM-004 | MUST | Action texts adapt to the target: the Move action's text is the delegate's action name (`Move Class...`, `Move Classes...`, `Move Package or Directory...`, `Move Packages or Directories...`, `Move Classes and Packages...`, `Move Members...`, `Move Inner Class...`, `Move Files...`, `Move File...`, `Move Directory...`, `Move Instance Method...` for Java; Kotlin: `Move declarations`), Copy's text is `Copy Class...`, `Copy Classes...`, `Copy File...`, `Copy Files...`, `Copy Directory...` and so on, Inline's text is `Inline Method...`, `Inline Field...`, `Inline Parameter...`, `Inline Super Class...`. | Caret on a class name: Refactor > Move reads `Move Class...`; select two directories in the Project view: it reads `Move Directories...`. |
| REF-FRM-005 | MUST | Before any handler runs the framework (1) commits all documents to PSI (if this is cancelled the refactoring is not started), (2) finishes an active completion popup as if the normal selection key was pressed, (3) runs the auto-import of an unresolved reference at the caret (so that a reference that only needs an import is resolved before the refactoring analyses it), (4) invokes the handler with the editor, file and data context; without an editor the handler gets the selected elements. When no handler matches the context the error "Cannot perform refactoring.\nCaret should be positioned at symbol to be refactored" is shown (see REF-FRM-008). | Type `Lis<caret>` with a lookup open and press Shift+F6: the lookup is finished first. |
| REF-FRM-006 | MUST | `Refactor This...` (`Ctrl+Alt+Shift+T`) opens a list popup titled `Refactor This` at the best position for the editor (or in the middle of the frame without editor). The list is the flattened Refactor menu: only refactorings that are enabled and visible for the current context are present; each submenu becomes a separator row carrying the submenu's text followed by its items and a plain separator; items show their shortcut. Choosing an item runs that action. The action is visible only in the main menu, in Search Everywhere and when invoked by shortcut. | Caret on a field: popup shows Rename, Change Type Signature..., the `Extract/Introduce` group with Introduce Variable and others, Inline, Move, Copy, Safe Delete, Encapsulate Fields, ...; no disabled rows. |
| REF-FRM-007 | MUST | While an in-place refactoring is active, starting another refactoring that does not continue the same element is refused. If the active session belongs to the same document the refusal is an editor error hint `{command} is not finished yet.` (command is the name of the active command, for example `Rename`). If it belongs to another document a modal question appears with the title `Cannot perform refactoring.`, the same message and the buttons `Continue Started` (navigate to the active session) and `Abandon Started` (cancel it, restoring its text per REF-FRM-052, and open the other document's session state as abandoned). Invoking Rename again on the element of an active in-place rename switches to the dialog form (REF-REN-014). | Start in-place rename on a local variable, press Ctrl+Alt+V: hint `Rename is not finished yet.` and the template stays active. |

### 1.1 Inventory coverage (actions of the area)

Every action registered in the area ("Refactorings", 53 actions in `inventory/refactorings.md`)
is covered or explicitly placed:

| Action id | Covered by |
|---|---|
| `Refactorings.QuickListPopupAction` | REF-FRM-006 |
| `RenameElement`, `RenameFile` | `02-rename.md` |
| `Move`, `CopyElement` | `03-move.md`, REF-OTH-040 |
| `IntroduceVariable`, `IntroduceConstant`, `IntroduceField`, `IntroduceParameter`, `ExtractMethod`, `IntroduceFunctionalParameter`, `IntroduceFunctionalVariable` | `04-extract-introduce.md` |
| `IntroduceParameterObject`, `ExtractClass` (Extract Delegate), `ReplaceMethodWithMethodObject` | `04-extract-introduce.md` (REF-EXT-080..090), the latter two LATER |
| `Inline` | `05-inline.md` |
| `ChangeSignature`, `ChangeTypeSignature` (Type Migration) | `06-change-signature.md`, REF-OTH-020 |
| `SafeDelete`, `MembersPullUp`, `MemberPushDown`, `ExtractInterface`, `ExtractSuperclass`, `EncapsulateFields`, `AnonymousToInner`, `MakeStatic`, `ConvertToInstanceMethod`, `InvertBoolean`, `TurnRefsToSuper`, `InheritanceToDelegation`, `MethodDuplicates` | `07-safe-delete-and-others.md` |
| Kotlin: `ExtractFunction`, `ExtractFunctionToScope`, `IntroduceProperty`, `IntroduceTypeAlias` | `04-extract-introduce.md` |
| `Generify`, `RemoveMiddleman`, `WrapReturnValue`, `ReplaceTempWithQuery` (rare refactorings plugin), `I18nize` | `07-safe-delete-and-others.md` REF-OTH-090 (LATER: rarely used; plugin) |
| `ExtractInclude`, `ExtractModule`, `ExtractModuleFromPackage` | LATER, REF-OTH-091 (include files are for template languages; module extraction needs the module model, see Gradle/Maven specs) |
| `MoveLineUp/Down`, `MoveStatementUp/Down`, `MoveElementLeft/Right` | not refactorings: editor actions, belong to the editing-actions area (inventory `editing-actions.md`) |
| `MoveEditorToOppositeTabGroup`, `MoveTabDown/Right`, `MoveToolWindowTab*`, `MoveWindowBuiltinDisplayAction`, `MoveMinimap` | not refactorings: window and editor-tab actions (`window-ui.md`) |
| `ConvertTo{Unix,Mac,Windows}LineSeparators` | not refactorings: line separator actions (`file-ops.md`) |
| `Unwrap` (`Ctrl+Shift+Delete`) | not a refactoring: code unwrap, belongs to editing/error catalog (JAVA-ERR-120) |
| Settings: `Editor | General | Refactorings` options | REF-FRM-050 |

## 2. Handler pipeline of processor-based refactorings

Rename, Move, Safe Delete, Inline, Change Signature, Pull Up, Push Down, Extract Interface,
Extract Superclass, Encapsulate Fields, Convert Anonymous to Inner, Introduce Parameter and
Copy (partially) share one pipeline (the "refactoring processor"):

1. **Dialog** (or in-place UI) collects options. The dialog's `Refactor` button (mnemonic R, the
   default button) runs the pipeline; `Preview` (mnemonic P) runs it in preview mode; `Cancel` closes. Dialogs
   validate on every change: while the input is invalid the Refactor and Preview buttons are
   disabled and the error is shown at the bottom of the dialog.
2. **Prepare**: commit all documents; run **find usages** in a modal progress titled
   `Looking for Usages`, cancellable; the work runs in a read action. Errors: if usages in a
   language that cannot be refactored are found, the error "Unsupported references found in {n} files" (n = language name for `{0}`:
   `Unsupported references found in {Language} files`) is shown and the refactoring stops; in
   dumb mode `Refactoring is not available until indexes are ready` (notification) and stop; when
   the search was cancelled by the user, nothing happens.
3. **Preprocess usages**: the refactoring detects *conflicts* (REF-FRM-020..) and may ask
   questions (overriding methods, related files, getters/setters).
4. **Decide preview**: preview is shown when the user pressed `Preview`, or when a usage lies in
   a read-only file that could not be made writable (REF-FRM-031), or when elements belong to
   unloaded modules. Otherwise go to 5.
5. **Execute**: one *command* named by the refactoring (`Rename: Foo -> 'Bar'`, `Moving ...`,
   `Deleting ...`, see each file) around the whole change; all files change under one write action
   with a cancellable modal progress; Undo reverts the whole refactoring in one step, in every file
   (REF-FRM-040).
6. **Post-processing**: refactoring helpers run on the touched files (REF-FRM-045).

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| REF-FRM-010 | MUST | The pipeline above, with find-usages progress text `Looking for Usages`, in a read action, cancellable; edits in a single command (single undo); progress titles for the phases `Detecting Possible Conflicts...`, `Checking Conflicts...`, `Processing {0}`. A cancelled search leaves the code unchanged. | Rename a class used in 3 files: a single Ctrl+Z restores all 3 files. Cancelling the progress changes nothing. |
| REF-FRM-011 | MUST | Documents are committed before analysis; a refactoring never works on stale PSI. If the code changed between the usage search and the preview's `Refactor` button, the preview refuses with "Cannot perform refactoring operation.\nThere were changes in code after usages have been found.\nPlease perform usage search again." and offers a rerun. | Open a rename preview, edit the file, press Refactor: that message. |
| REF-FRM-012 | MUST | Read-only handling (see REF-FRM-030..032): refactoring elements that are not writable are not changed silently: "{0} is read-only." / "{0} is located in a read-only container." (for library jars) and the refactoring is refused with the title `Refactoring cannot be performed` (dialog) or as editor hint. | Rename a method of a library class: error `Method 'X.y()' is located in a read-only container.` |
| REF-FRM-013 | MUST | Refactoring dialogs share conventions: title is the refactoring name (`Rename`, `Move`, `Safe Delete`, `Change Signature`, ...); Enter triggers `Refactor` unless a table is being edited; Escape cancels; the last used option values are remembered per refactoring and per element kind (see each file for defaults); checkbox mnemonics are those in the texts below. | Reopen a refactoring dialog after changing a checkbox: it shows the new value. |

## 3. Conflicts

A *conflict* is a problem the refactoring detects before changing code that would make the result
not compile or change behavior. Conflicts are collected as pairs (code element, message); the
same message text is shown once.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| REF-FRM-020 | MUST | Default conflicts dialog ("Problems Detected"): modal, title `Problems Detected`, resizable (content 500 x 400 area), label at the top `The following problems were found:`, below a scrollable HTML pane with the messages separated by a blank line, in the order collected, at most 20 messages; if more exist the pane ends with a link `Show more...` that, when clicked, replaces the text with all messages. Bottom label `Do you wish to ignore them and continue?` (shown when the OK button is enabled). Buttons in order: `Continue` (OK), `View Usages` (shown when the conflicts are bound to elements), `Cancel`. When the View Usages button is present the default button is `Cancel` (Enter cancels; Continue must be pressed explicitly). `Continue` proceeds with the refactoring and ignores all conflicts; `Cancel` aborts without changes. | Rename a method to a name that already exists with the same signature: dialog lists `Method bar(int) is already defined in class A`; Enter aborts, `Continue` renames. Cases REF-FRM-020 case 1-3. |
| REF-FRM-021 | MUST | `View Usages` closes the dialog, aborts the refactoring and opens the Find tool window tab `Refactoring Preview` (code usages node text `References in code ...`) listing the conflict elements; the tab has the actions `Do Refactor`-style button text `Refactor` and `Cancel`, and `Rerun` (REF-FRM-033). | Click View Usages: tool window opens with conflict usages; the dialog is gone. |
| REF-FRM-022 | SHOULD | Updated conflicts dialog (registry `refactorings.use.updated.conflicts.detected.dialog`): title `Conflicts Detected`, OK button `Refactor Anyway`, standard Cancel, compact style; content is a vertical splitter (40 percent top): tree of conflicts (grouped by the usual usage-group toolbar: Group By, expand all, collapse all, previous/next conflict with the standard occurrence shortcuts) above a code preview with the file name and relative path as the title; a right-aligned label `{n} conflicts`; the first conflict is selected and previewed; tree has the focus; a left-side button `Show Conflicts in View` opens the Find tab like View Usages (`Conflicts` tab name). | Open with 3 conflicts: label `3 conflicts`; selecting a node scrolls the preview to the element. |
| REF-FRM-023 | MUST | Conflicts abort the refactoring in headless/test mode by raising an error carrying all messages (no UI). The implementation exposes the message list for tests. | A test harness asserts the messages of a rename conflict. |
| REF-FRM-024 | MUST | Every conflict message uses the exact texts of the catalogs in the per-refactoring files. Element descriptions inside messages are `<kind> <name>` where the name is shown in bold monospace (plain text in tests): kinds `field`, `enum constant`, `method`, `constructor`, `parameter`, `local variable`, `pattern variable`, `package`, `class`, `interface`, `enum`, `record` (`local class`, `local interface` ... for classes declared in a method body), `anonymous class derived from X`, `class body of <enum constant>`, `static initializer of <class description>`, `instance initializer of ...`. A method's name is shown with its parameter types `bar(int, String)`; with "include parent" the containing class is prefixed `Foo.bar(int)`. No quote characters are used (`class A`, not `class 'A'`). Messages starting with the description get the first letter capitalized. | Message for a clashing method: `Method bar(int) is already defined in class A`. |

## 4. Preview in the Find tool window

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| REF-FRM-030 | MUST | Preview shows a usage view tab titled `Refactoring Preview` in the Find tool window with: a targets node `{Elements header}` (for example `Class to be renamed...`: each refactoring supplies its own header text), a node `References in code ...` (code usages, with file count) and a node `Occurrences found in comments, strings and non-code files ...` (non-code usages, when requested), the usages count string `usages`, excluded usages are not changed. Items are checkable via the "Exclude" action (`Delete` key excludes, `Insert` includes). The bottom button is `Refactor` (performs the refactoring on all non-excluded usages), plus `Cancel` and `Rerun`. Read-only files' nodes are shown in red. | Rename with Preview: tree grouped by file; exclude one usage; press Refactor: that usage keeps the old name. |
| REF-FRM-031 | MUST | If any usage or element is in a read-only file, the framework first asks the version-control/read-only handler to make files writable (for example `Clear read-only status` prompt of the VCS integration; files under version control that need checkout are checked out). If files stay read-only the refactoring does not change them silently: preview is forced and the status bar shows `Occurrences found in read-only files`. In the preview the user can continue (read-only usages are skipped, not changed). | Rename a class with a usage in a read-only test file: preview opens, status text as above. |
| REF-FRM-032 | SHOULD | The status bar shows the result of non-code replacement: `{n} occurrence(s) changed` / `No occurrences found` where applicable (rename of non-code usages). | After rename with "Search for text occurrences" the status text appears. |
| REF-FRM-033 | MUST | `Rerun` in the preview tab re-executes the usage search with the same parameters and replaces the preview content. A tab for the same refactoring reuses the existing tab. | Press Rerun after editing; the list updates. |

### 4.1 Search in comments and strings / Search for text occurrences

Many dialogs have two checkboxes with these exact texts (mnemonics C and T):
`Search in comments and strings` and `Search for text occurrences`. Semantics:

- *Comments and strings*: occurrences of the old name (or qualified name) in comments, doc comments
  and string literals **of code files of the same language family**, as whole words (the old
  name is matched as an identifier token; `Foo` does not match inside `FooBar`).
- *Text occurrences*: occurrences in **non-code files** (XML, properties, JSON, plain text, build
  files...) and, for files of unknown types, as whole words, honoring the project's file type
  associations. The checkbox is hidden and unselected for elements where it is meaningless
  (local variables, parameters; also in dumb mode `Search for references` variants are disabled
  with `(not available during the project analysis)`).
- Both kinds of usages are shown in the preview in their own node and are replaced by the new
  name; in a qualified-name context (inner classes use the `$` form in strings) the replaced
  text is the qualified name after rename.
- The default values per element kind live in the refactoring settings (see REF-REN-040) and
  are stored application wide.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| REF-FRM-035 | MUST | The checkboxes exist with the texts above in Rename, Move (class/package), Safe Delete (`Search in comments and strings`, `Search for text occurrences`, plus `Safe delete (with usage search)` when the dialog is the delete confirmation), Inline method/field/class (comments only). Matching is by whole-word identifier. Non-code occurrences are listed in the preview under their own node and replaced when the refactoring proceeds; unselecting hides the node. | Rename class `Foo`; a string `"a.Foo"` in a `.properties` file and a comment `// Foo here` change when both boxes are on; a variable `FooBar` in a comment does not. |

## 5. Undo, commands and listeners

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| REF-FRM-040 | MUST | Undo: one undo step reverts the complete refactoring in all files, including the file/directory renames and moves it performed (and re-creates deleted files). Redo re-applies. When the refactoring was started from a tree without an editor the undo is a *global* (project-wide) undo and asks for confirmation as usual for global undo. An in-place refactoring in a single file produces one undo step for the template session plus one for the refactoring part; cancelling the template (Esc) reverts all text typed in the template (REF-FRM-052). | Rename a class used in 3 files, press Ctrl+Z once: all three files and the file name are restored. |
| REF-FRM-041 | SHOULD | Refactoring start/finish/conflict events are published to listeners by a stable string id: `refactoring.rename`, `refactoring.inplace.rename`, `refactoring.move`, `refactoring.safeDelete`, `refactoring.inline.local.variable`, `refactoring.inline.pattern.variable`, `refactoring.extract.method`, `refactoring.extractVariable`, `refactoring.introduce.parameter`, `refactoring.changeSignature`, ... Plugins (for example test run configurations, "Rename Test Methods") get the element before and after. | A listener registered for `refactoring.rename` receives before/after element data. |
| REF-FRM-042 | SHOULD | After the refactoring the suggested-refactoring tracking state is reset (so a gutter suggestion for the same edit disappears). | Rename via dialog after a suggestion appeared: suggestion gone. |

## 6. Post-processing helpers

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| REF-FRM-045 | SHOULD | After a Java refactoring that changed references (rename of anything that is not confined to local scope, move, inline, change signature, safe delete) the files that were touched (moved/renamed element files plus files with changed code usages) are searched for **redundant imports** with a progress dialog titled `Removing redundant imports` (progress text2 = file path, not cancellable) and those imports are deleted: an import of a resolvable class that is now redundant (same package, `java.lang`, duplicate) is removed; an unresolvable import is kept (except exact duplicates). This is not a full Optimize Imports: ordering and unused-but-valid imports are untouched. A rename confined to local variables/parameters is skipped. Kotlin has an equivalent helper that optimizes imports of changed files. | Class `b.Y` imports `a.X`; move `a.X` to package `b`: `Y` ends without any import for `X` (case REF-FRM-045 case 1). |

## 7. In-place refactoring (template based)

In-place mode edits the code directly in the editor using a *template session*: the name (or
introduced element) is shown as an editable field with the other occurrences linked and
highlighted, and the actual refactoring (usage search, conflicts, other files) runs when the user
accepts. It is the default (`Editor | General | Refactorings`: `Specify refactoring options: In the editor`).

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| REF-FRM-050 | MUST | Settings page group `Refactorings` in `Editor | General`: radio group `Specify refactoring options:` with `In the editor` (default) and `In modal dialogs`; checkbox `Preselect current symbol name for Rename refactoring` (default on); checkbox `Show inline dialog for local variables` (default on, controls the Inline Variable popup). With `In modal dialogs` every refactoring that has both forms uses its dialog, also Extract Method and Introduce Variable (no template). | Switch to modal dialogs; Shift+F6 on a local variable opens the Rename dialog. |
| REF-FRM-051 | MUST | Template session UX: the editable segment is highlighted as a template field; typing replaces the preselected name (when `Preselect current symbol name` is on, the whole name is selected at start, otherwise the caret is at the end of the name); all other occurrences update live (identical text, including references in other places of the same file that are linked); occurrences linked are highlighted with the standard refactoring-occurrence attribute; `Enter` or `Tab` finishes (`Next Template Variable or Finish In-Place Refactoring`, keys `Tab` and `Enter`), `Escape` cancels. A lookup popup under the name lists suggested names (Rename: the old name first, then the name suggestions; Introduce: the type-based name suggestions); `Up/Down` navigate, `Enter` chooses (a chosen name replaces the field). The advertisement line at the bottom of the lookup: `Press Tab to show options popup` hint when an options inlay exists (otherwise `Press Ctrl+Shift+F6 ...`-style hint for shortcut of the dialog: `Press {0} to show dialog with more options`, where {0} is the shortcut of the full action, e.g. `Shift+F6`). `Press {0} again to perform refactoring.` appears as a hint when the user triggers the refactoring shortcut again while the template is active. | Start in-place rename, type, press Enter: all occurrences renamed and the template ends; press Escape: the old name is restored everywhere (REF-FRM-052). |
| REF-FRM-052 | MUST | Cancel semantics: Escape cancels the session and restores the original code exactly, in every segment (rename) or removes the introduced declaration and restores the expression (introduce variable/constant/field/parameter), and the undo stack returns to its pre-start state. Enter with the unchanged name just ends the session without a command. Enter with an *invalid* identifier (or an empty one) does not refactor: a small confirmation popup titled `Inserted identifier is not valid` with the buttons `Continue editing` and `Cancel` appears; `Continue editing` restores the text and restarts the in-place session with the typed text preserved in the field, `Cancel` rolls the text back to the old name and ends the session. Enter with a name that causes an unresolvable collision (REF-REN-101) shows a confirmation popup titled by the collision (for example `Variable 'y' Already Exists`) with the buttons `Refactor anyway` and `Continue editing`, the conflicting element is highlighted while the popup is shown. | Type `1x`, press Enter: popup `Inserted identifier is not valid`; Cancel: name back to the old one. |
| REF-FRM-053 | MUST | The options popup of an in-place session: an inlay button after the edited name (gear-style, tooltip header `Choose where to rename occurrences in addition to usages:` followed by the lines `– In comments and string literals` and `– In files that don’t contain source code`, and `Use {shortcut} to navigate to these options`). `Tab` (`Alt+Shift+O`, action `Show Options Popup for In-place Refactoring`) moves the focus to the inlay and opens a popup panel titled `Also rename in:` with checkboxes `Comments and strings` and `Text occurrences` (the latter only when text occurrences are meaningful for the element) and a link `More options...` that switches to the full dialog (REF-REN-014). Changing a checkbox updates the saved default for this element kind and restarts the session keeping the typed name. `Enter` inside the popup closes it, `Esc` closes it and keeps the session. Introduce Variable's options popup has `Declare final` and `Declare var type` (REF-EXT-005); Extract Method's has `Annotate`, `Make static` / `Make static and pass fields` and the link `More options` (REF-EXT-050). | Press Tab in a member rename: the popup appears with the two checkboxes. |
| REF-FRM-054 | SHOULD | Only one in-place session per editor. Starting a second one while the first is on another element asks (REF-FRM-007); `Navigate to Started` jumps to the first session's range when the first session lives in another document. Dumb mode: a member in-place rename that needs the usage search starts the refactoring in dumb-aware mode and shows `Refactoring is not available while indexing is in progress` (title `Indexing`) if indexes are not ready when Enter is pressed. | Press Enter during indexing: the notification, the template ends without renaming other files. |

## 8. Suggested refactorings (gutter / intention while typing)

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| REF-FRM-060 | SHOULD | When the user edits a declaration name directly in the editor (not through a refactoring) the IDE offers to rename the usages: a gutter icon and an intention `Rename usages of '{old}'`; the popup text `Rename usages of '{old}' to '{new}'`, tooltip `Rename usages of '{old}' to '{new}'... {shortcut}`; buttons in the popup `Rename` and `Rename with Preview` and `Ignore`. When text occurrences exist in comments or strings, after accepting an extra question dialog `Rename text occurrences` with the text `Found occurrences in comments and strings while renaming '{old}' to '{new}'. Would you like to rename them as well?` and the choices `Rename code occurrences` / `Rename all occurrences`. | In Java rename a method name by typing; gutter suggestion appears after the daemon runs; `Rename` renames the callers. |
| REF-FRM-061 | SHOULD | The same mechanism offers `Update {callers/usages/overrides/implementations} to reflect signature change` when parameters of a method are edited (intention `Update {0} to reflect signature change`, gutter tooltip `Update {0} of '{1}' to reflect signature change... {2}`, popup `Update {0} to reflect signature changes:`), with an input table for new parameter values (`Add values for new parameters:` placeholder `Optional value`, checkbox `Use any variable` with hint `If a single variable of suitable type exists at the call place, it will be used as argument value`), buttons `Update`, `Next`, `Ignore`, or when nothing can be updated the tooltip `There are changes in signature but nothing can be updated in usages`. | Add a parameter to a method signature by typing; accept the suggestion; callers get the new argument. |

## 9. Open questions

1. The default of the registry key `refactorings.use.updated.conflicts.detected.dialog` (legacy
   dialog `Problems Detected` vs new `Conflicts Detected`) was not confirmed from resources. The
   legacy dialog is specified as the default (MUST), the new one as SHOULD.
2. The options popup of in-place Rename is gated by the registry key `enable.rename.options.inplace`
   (default true in the source read); when false the `Press Shift+F6 to show dialog` advertisement
   replaces the inlay (REF-FRM-051).
3. Whether Escape in in-place Introduce Variable restores the original expression is inferred from
   the shared template cancel path (REF-FRM-052); verify in the running reference product.

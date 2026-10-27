# Workbench spec 08: file operations, scratches, templates, encoding and line separators

Status: specification (clean-room). ID prefix `WB-FILE`. Priorities `MUST` / `SHOULD` / `LATER` as in `docs/specs/README.md`.
Executable cases: `cases/08-file-operations-and-scratches.cases.md`.

Scope: the user-facing file operations of the main menu and editor: Save All, Save File, Save As, Reload from Disk and the external-change conflict dialog, scratch files and buffers, file templates and
New File from template, encoding and line-separator **settings** and conversions, BOM, read-only toggle, and the file-operation actions listed in the inventory. VFS and document **mechanics** (events, commit model, autosave
timing, conflict detection) are in `docs/specs/platform/01-vfs-and-documents.md` (PLAT-VFS) and the saving spec `platform/09-saving-and-autosave.md` (branch `docs/specs-platform` or later); this file specifies only what the user sees and does.

Sources studied: `ic: platform/scratch/src/com/intellij/ide/scratch/ScratchFileActions.java`, `ScratchRootType.java`, `ic: platform/lang-impl/src/com/intellij/ide/scratch/ScratchFileCreator.java`, `ScratchFileCreationHelper.java`, `ScratchTreeStructureProvider.java`,
`ic: platform/lang-impl/src/com/intellij/ide/actions/CreateFileFromTemplateAction.java`, `CreateFileFromTemplateDialog.java`, `CreateFileAction.java`, `ic: platform/platform-impl/src/com/intellij/openapi/vfs/encoding/ChangeFileEncodingAction.java`, `EncodingUtil.java`,
`ic: platform/platform-impl/resources/idea/PlatformActions.xml`, `LangActions.xml`, `ic: platform/platform-resources-en/src/messages/ActionsBundle.properties`, `ic: platform/platform-api/resources/messages/IdeBundle.properties`, `ApplicationBundle.properties`, `UIBundle.properties`, `ic: platform/platform-resources/src/keymaps/$default.xml`, inventory `file-ops.md`.

## 1. Menu and shortcuts

File menu groups (reference order, as far as the file-ops actions are concerned): New, Open..., Open Recent, Close Project, then settings groups, **File Properties** (File Encoding, Associate with File Type..., Toggle Read-Only Attribute, Line Separators, Add BOM, Remove BOM), `---`,
**Save All** (Ctrl+S), **Reload All from Disk** (Ctrl+Alt+Y), Invalidate Caches..., Manage IDE Settings, Save File as Template..., Export, Print..., Exit.

| Action id | Text | Default shortcut (Win/Linux) | Notes |
|---|---|---|---|
| `SaveAll` | Save All (description "Save all files and settings") | Ctrl+S | also in main toolbar |
| `SaveDocument` | Save File (description "Save only the file opened in the current editor") | none | |
| `SaveAs` | Save As... (description "Save the selected file under a new name") | Ctrl+Shift+S | |
| `Synchronize` | Reload All from Disk (description "Detect all externally changed files and reload them from the disk") | Ctrl+Alt+Y | also in toolbar |
| `SynchronizeCurrentFile` | Reload from Disk | none | in project view and navigation bar popup menus |
| `Refresh` | Refresh | Ctrl+F5 | context dependent (rerun, refresh a view) |
| `ForceRefresh` | Force Refresh | Ctrl+Shift+F5 | re-reads VFS for the selection |
| `InvalidateCaches` | Invalidate Caches... | none | |
| `NewScratchFile` | Scratch File | Ctrl+Alt+Shift+Insert | in New group |
| `NewScratchBuffer` | New Scratch Buffer | none | hidden when buffers are disabled |
| `SaveFileAsTemplate` | Save File as Template... | none | File menu |
| `ToggleReadOnlyAttribute` | Toggle Read-Only Attribute | none | File Properties |
| `ChangeFileEncodingAction` | File Encoding | none | File Properties |
| `AddBom`, `RemoveBom` | Add BOM, Remove BOM | none | File Properties and the encoding widget popup |
| `AssociateWithFileType` | Associate with File Type... | none | file context menus |
| `OpenInBrowser` | Open in Default Browser | none | |
| `OpenInAssociatedApplication` | Open in Associated Application | none | Reveal group |
| `RevealIn` | Show in File Manager | none | Reveal group |
| `CopyPaths`, `CopyAbsolutePath` (Absolute Path), `CopyFileName` (File Name), `CopyReference` | Copy Paths / Absolute Path / File Name / Copy Reference | Ctrl+Shift+C / - / - / Ctrl+Alt+Shift+C | Copy Path/Reference group |
| `Exit` | Exit | none | File menu |
| `ExportToHTML`, `ExportToTextFile`, `Print` | Export Files or Selection to HTML... (menu: "Files or Selection to HTML..."), Export to Text File (Alt+O in usage views), Print... | none / Alt+O / none | LATER |

## 2. Save, reload, external changes

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-FILE-001 | MUST | **Save All** (Ctrl+S) writes every modified document to disk (and project/IDE settings), using each file's encoding and line separator and the "Back up files before saving" safe-write option; documents unchanged are not rewritten; it does nothing visible when nothing is modified. The tab dirty marker disappears. | Edit A and B, Ctrl+S: both files on disk updated; no dirty markers. |
| WB-FILE-002 | MUST | **Save File** saves only the document of the current editor. | Edit A and B, run Save File in A: only A saved. |
| WB-FILE-003 | SHOULD | **Save As...** writes the current document to a new path chosen in a chooser (title "Save as"), opens the new file and keeps the old on disk; existing target asks to overwrite. | Save As `b.txt`: editor now shows `b.txt`, `a.txt` unchanged on disk. |
| WB-FILE-004 | MUST | Saving a file that cannot be written (read-only, no permission, disk error) shows the dialog **"Cannot Save Files"** with the text "Following errors occurred on attempt to save files:" and the error lines, buttons **Revert Changes** (reload from disk, dropping the edit) and **Ignore Changes** (keep the edit in memory; the document stays dirty). | Make the file read-only on disk after opening, edit, Ctrl+S: dialog; Ignore Changes: tab stays dirty. |
| WB-FILE-005 | MUST | **Reload All from Disk** (Ctrl+Alt+Y) detects every externally changed file in the project and reloads it; unmodified editors update silently; modified editors with external changes raise the conflict dialog (WB-FILE-007). | Change `a.txt` externally, Ctrl+Alt+Y: editor shows the new text without prompts. |
| WB-FILE-006 | MUST | **Reload from Disk** on a selected file or directory (project view menu) re-syncs just that subtree. | Select `docs/` and Reload from Disk: only changes under `docs/` are picked up. |
| WB-FILE-007 | MUST | **File Cache Conflict**: when a document has unsaved memory changes **and** the file changed on disk, the dialog titled "File Cache Conflict" (with the file name appended when shown per file: "File Cache Conflict {name}") says "Changes have been made to '{file}' in memory and on disk." with the buttons **Load File System Changes** (take disk, drop memory), **Keep Memory Changes** (keep memory, ignore disk until next save), **Show Difference** (diff with sides "File system content" and "Memory content"; in the diff window a **Save Memory Content** button). Where an automatic merge is possible the extra action "Merge External Changes" is offered. | Edit in memory, change on disk, activate the IDE: dialog with the three buttons. |
| WB-FILE-008 | MUST | When a document is **unmodified** and the file changed on disk, the editor reloads silently (a "File was changed on disk." / "Reload" balloon is shown only when automatic reload is disabled). | Unmodified editor and external change: text updates, no dialog. |
| WB-FILE-009 | MUST | Synchronization triggers: on IDE window activation and on opening an editor tab (setting "Sync external changes: When switching to the IDE window or opening an editor tab", default on), and periodically while inactive (experimental, off), plus the explicit actions above. | With the option on, change a file externally and activate the window: text updates. |
| WB-FILE-010 | MUST | `Refresh` (Ctrl+F5) acts on the current context (in the project view: refresh the selection from disk) and `Force Refresh` (Ctrl+Shift+F5) forces a full re-read of the selection even if timestamps are unchanged. | Touch a file keeping the same timestamp but different content: Force Refresh picks it up, Refresh may not. |
| WB-FILE-011 | SHOULD | **Invalidate Caches...** opens a dialog (title "Invalidate Caches") explaining "Remove caches and indexes for all projects. New caches will be built when you reopen the projects." with options and an action button; restarts the IDE. | Menu File > Invalidate Caches...: dialog shown. |
| WB-FILE-012 | SHOULD | **Open in Default Browser**, **Open in Associated Application** and **Show in File Manager** hand the file to the OS; unsupported platforms show "Cannot open '{0}' in a file manager: the action is not supported on this platform." | Linux: Show in File Manager opens the file manager at the file. |

## 3. Scratch files and buffers

Scratch files are real files in the IDE's scratch directory (`{config}/scratches`), not part of any project, shown under **Scratches and Consoles** in the Project view (`02-project-view.md`), searchable and with the language of their extension (or an explicit language mapping).

**New Scratch File** (`NewScratchFile`, Ctrl+Alt+Shift+Insert; menu text "Scratch File" inside the New popup, "Create Scratch File" in the editor popup with a selection, "New Scratch File" otherwise): opens a popup titled **"New Scratch File"** that lists languages (most recently used first, a pinned "Plain text" when configured) and the entries
"From selection ({type})" (when text is selected in an editor; the file starts with the selection and takes the type of the source file) and "From selected view" (for text extracted from a tree/list). Choosing a language creates the file, opens it in the editor with the caret at 0 (or at the end of inserted text as defined by the language helper).

File naming: `scratch` + extension of the language's default extension, `scratch.txt`, `scratch.java`; if the name exists in the target directory the next free **`scratch_N.ext`** with N = 1, 2, ... is used (create-new-always policy). Created inside the scratch directory of the target root, or in the subdirectory selected in the Project view's Scratches node.

**New Scratch Buffer**: creates `buffer{N}.txt` where N cycles `1..B` (B = registry `ide.scratch.buffers`, default not read) and **reuses** the existing buffer file if present (create-if-missing); hidden when B is 0.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-FILE-020 | MUST | Ctrl+Alt+Shift+Insert opens the "New Scratch File" language popup; choosing a language creates `scratch.<ext>` in the scratch directory and opens it. The action is available in a project window from the main menu, the New popup, the Search Everywhere ("Create New Scratch File"), and when the editor has selected text. | Press the shortcut, choose "Plain text": `scratch.txt` open. |
| WB-FILE-021 | MUST | File name uniqueness: `scratch.txt`, then `scratch_1.txt`, `scratch_2.txt` ... for the same extension. | Create two plain-text scratches: `scratch.txt` and `scratch_1.txt`. |
| WB-FILE-022 | MUST | With a non-empty selection in an editor, the popup's first entry is "From selection ({type})" and creates a scratch containing exactly the selected text with the source file's language. | Select `int a;` in Java, choose "From selection (Java)": `scratch.java` contains `int a;`. |
| WB-FILE-023 | MUST | Scratch files are editable like normal files, are saved with Save All, survive restarts, are listed under "Scratches and Consoles", and are excluded from project search scopes except the "Scratches and Consoles" scope. | Create, type, restart: file and content remain. |
| WB-FILE-024 | MUST | **Change File Language** (`Scratch.ChangeLanguage`, shown in the editor popup and the project view popup for scratch files) opens a popup "Change Language" with entries "Change Language ({current}) to..."; choosing a language sets an explicit language mapping for the file (the file name keeps its extension unless the language provides another) and reparses. Dialect variants read "Change Dialect ({x}) to...". | In `scratch.txt`, choose Java: the editor highlights as Java; the mapping persists. |
| WB-FILE-025 | SHOULD | **Show Scratch Files** (`Scratch.ShowFilesPopup`) opens a recent-locations style popup titled **"Scratch Files"** (changed-only toggle: "Recently Edited Scratch Files"), listing scratch files sorted by name (natural order, at most 1000), each with the file name and a code preview; empty text "No scratch files found"; Delete removes the selected scratch. | With no scratch: "No scratch files found". |
| WB-FILE-026 | SHOULD | **Export to Scratch File** (`Scratch.ExportToScratch`) turns the text of a non-editor view (tree, list, console output) into a plain-text scratch file; disabled for real file editors. | In a console, run Export to Scratch File: `scratch.txt` with the console text. |
| WB-FILE-027 | MUST | The creation is one undoable command named "Create New Scratch File" (global undo). | Ctrl+Z immediately after creation removes the file. |
| WB-FILE-028 | SHOULD | New Scratch Buffer reuses `buffer{N}.txt` as above. | Run New Scratch Buffer twice: same file opened. |
| WB-FILE-029 | MUST | The scratch directory is created on demand; if the file cannot be created the error dialog says "Could not create file '{name}'" (title "Error"). | Make the directory read-only: dialog with the message. |

## 4. File templates and New File from template

**File and Code Templates** (Settings > Editor > File and Code Templates, tabs **Files**, **Includes**, **Code**, **Other**): a list of templates; each template has **Name**, **Extension**, a text body (with `${VAR}` substitutions), a **Description** area and two options "Reformat according to style" and "Enable Live Templates".
List actions: Create Template, Create Child Template File, Copy Template (copy name "Copy{N} of {name}"), Remove Template, Revert to Original Template (confirmation "Reset to original template?", title "Reset Template"). A customized built-in template is flagged "* Customized". Errors: "Specify a template name", "Invalid template file name or extension", "Unable to parse template '{0}' / Error message: {1}".

Substitution: `${NAME}` (the entered name), `${PACKAGE_NAME}`, `${USER}`, `${DATE}`, `${TIME}`, `${YEAR}`, `${MONTH}`, `${DAY}`, `${PROJECT_NAME}`, `${FILE_NAME}` are documented variables; `#parse("File Header.java")` includes another template; `#if/#end` conditional blocks.
The engine is a template engine with a small directive set (see open question 3).

**New File from template** (`CreateFileFromTemplateAction`, shown as entries of the New group such as "Java Class", "Kotlin Class/File"): opens a popup titled **"New {kind}"** (for example "New Java Class") with a **Name** field ("Enter a new file name:"/label "Name:") and, when the action offers several **kinds** (Class, Interface, Record, Enum, Annotation, ...), a **kind** list below it
(label "Kind:"; tooltip "Pressing Up or Down arrows while in editor changes the kind"), Up/Down arrows in the name field change the selected kind; Enter creates.
Errors shown inline: empty name: "File name cannot be empty"; invalid name: "Incorrect name"; creation failure dialog: "Cannot Create File" / "Cannot Create Class" with the reason. The new file is created from the template in the selected directory (or directory chosen), opened in the editor, and selected in the Project view. A name with `/` or `.` follows the nested-directory/package rules of WB-PV-081..084.

**Save File as Template...** (`SaveFileAsTemplate`): saves the current file's text as a new template; asks for the template name and extension (dialog layout: open question 8), then adds it to the Files tab.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-FILE-040 | MUST | New File from template: a popup "New {kind}" with a Name field; Enter creates the file from the template with `${NAME}` bound to the entered name (extension appended from the template when the name has none); the new file opens in the editor. | Name `Foo` in a Java source root with the "Class" template: `Foo.java` containing `public class Foo {}`. |
| WB-FILE-041 | MUST | Kind switching with Up/Down in the Name field when several kinds are offered; the popup title changes with the kind. | Press Down in the Name field: kind changes from Class to Interface. |
| WB-FILE-042 | MUST | Errors: empty name keeps the popup open with "File name cannot be empty"; existing file with the same name: "A file with the name 'x' already exists". | Name `Foo` twice: second time the inline error appears. |
| WB-FILE-043 | MUST | Template variables listed above are substituted at creation time with the current values (DATE, TIME formatted per locale; YEAR four digits; USER = OS user name; PROJECT_NAME = project name). | `${YEAR}` in a header comment: the current year. |
| WB-FILE-044 | SHOULD | Settings > File and Code Templates: edit, copy, create, remove and revert templates with the listed texts; a parse error blocks saving the template with the error dialog "Cannot Save Current Template". | Introduce an unclosed `#if`: "Unable to parse template" error. |
| WB-FILE-045 | SHOULD | "Save File as Template..." creates a template from the file; the file's extension is the template extension. | Save `Main.java` as template: it appears in the Files tab. |
| WB-FILE-046 | LATER | Child templates, template data language, per-scheme templates (IDE vs project). | n/a |

## 5. Encoding, BOM and line separators (settings)

Settings > Editor > **File Encodings** (project scope, see `05-settings-dialog-and-keymap.md`):

| Setting | Default | Meaning |
|---|---|---|
| Global Encoding | `<System Default: {x}>` shown for the default option; the IDE default charset otherwise UTF-8 | encoding for files without a project/path override |
| Project Encoding | follows global | encoding for project files |
| Path-specific encodings (table "Path" / "Encoding") | empty | per file or directory overrides |
| Create UTF-8 files: | `with NO BOM` (alternatives: `with BOM`, `with BOM under Windows, with no BOM otherwise`) | BOM written when creating a UTF-8 file; helper text e.g. "{IDE} will NOT add UTF-8 BOM to every created file in UTF-8 encoding" |

Line separators: Settings > Editor > Code Style > General > **Line separator:** with values **System-Dependent** (default), **Unix and macOS (\n)**, **Windows (\r\n)**, **Classic Mac OS (\r)**; applies to **new files only**; existing files keep their separator.
The status bar widgets (see `04-status-bar.md`, WB-SB-030..043) change a file's separator or encoding; `File > File Properties > Line Separators` is the same group (title "Line Separators").

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-FILE-060 | MUST | A new file created by the IDE (New File, template, scratch) is written with the configured **new-file line separator** (System-Dependent = `\r\n` on Windows, `\n` elsewhere) and the project encoding (BOM per policy). | Setting Windows (\r\n): new `a.txt` containing a line break has bytes `0D 0A`. |
| WB-FILE-061 | MUST | An existing file keeps its detected separator and encoding when opened and saved. The separator is detected from the first line break; inside the editor the text always uses `\n`; at save the detected separator is applied to every line break (inconsistent files: open question 7). | Open a file whose first line break is `\n` and a later one `\r\n`: widget shows `LF`; after Save the file contains only `\n`. |
| WB-FILE-062 | MUST | Encoding detection order for an existing file: explicit per-path setting, BOM, declared encoding in content where the file type defines one, project encoding, global encoding. | A file with UTF-8 BOM and project encoding ISO-8859-1: shows `UTF-8`, tooltip reason "BOM". |
| WB-FILE-063 | MUST | **Add BOM** / **Remove BOM** (File Properties and the encoding popup): add or remove the byte order mark for UTF encodings; undoable; Remove BOM on a file whose encoding mandates the BOM fails with the notification group text "Failed to remove BOM" ("Was unable to remove BOM in {n} file(s)"; "These files have mandatory BOM:"). | Add BOM to a UTF-8 file: saved bytes start with `EF BB BF`. |
| WB-FILE-064 | MUST | **Toggle Read-Only Attribute** flips the OS write attribute of the file(s) (menu variants "Make file read-only"/"Make file writable", "Make Directory Read-Only" ...); the editor becomes read-only/writable at once and the widget lock follows (WB-SB-060). | Toggle on a writable file: editing is rejected. |
| WB-FILE-065 | MUST | The "Create UTF-8 files" setting texts and values are exactly those of the table. | The combo lists the three values. |
| WB-FILE-066 | MUST | **Associate with File Type...** opens a dialog (list of file types, with "Add" for patterns) and registers the file's extension or name pattern with the chosen type; the editor reparses with the new type. | Associate `*.conf` with "Properties": highlighting changes. |

## 6. File chooser actions (the Open/Save dialogs)

The IDE's own file chooser (used by Open, Save As, directory selection) has a toolbar with: **Parent Directory** (Backspace), **Previous Directory** / **Next Directory** (as Back/Forward), **Refresh** (as Reload All from Disk, Ctrl+Alt+Y), **Home Directory** (Ctrl+1), **Project Directory** (Ctrl+2), **Module Directory** (Ctrl+3),
**Desktop Directory** (Ctrl+D), **Root Directory** (Ctrl+Backslash), **New Directory...** (Alt+Insert, Ctrl+N), **New File...**, **Delete...** (Delete), and a settings menu: **Show Hidden Files and Directories**, **Show/Hide Path Input Field** (Ctrl+P), **Detect Project Directories** ("Allows the IDE to peek inside to detect project directories").

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-FILE-070 | SHOULD | The chooser provides the toolbar actions above with these texts and shortcuts; Home, Project and Module go to their directories; New Directory and New File create entries in the current directory and select them. | Ctrl+1: path field shows the home directory. |
| WB-FILE-071 | SHOULD | The path text field supports typing paths with completion; Tab completes; `~` expands to home. | Type `~/Doc` + Tab: completes to `~/Documents`. |
| WB-FILE-072 | LATER | WSL home entry (`FileChooser.GoToWslHome`, "Home Directory in WSL") and `FileChooser.LightEditGotoOpenedFile` ("Select Opened File"). | n/a |

## 7. Inventory coverage (`file-ops.md`, 56 actions)

| Disposition | Actions |
|---|---|
| Covered here | `SaveAll`, `SaveDocument`, `SaveAs` (via SaveAs row), `Synchronize`, `SynchronizeCurrentFile`, `Refresh`, `ForceRefresh`, `InvalidateCaches`, `NewScratchFile`, `NewScratchBuffer`, `Scratch.ChangeLanguage`, `Scratch.ExportToScratch`, `Scratch.ShowFilesPopup`, `SaveFileAsTemplate`, `AddBom`, `RemoveBom`, `ChangeFileEncodingAction`, `ToggleReadOnlyAttribute`, `AssociateWithFileType`, `RevealIn`, `OpenInAssociatedApplication`, `OpenInBrowser`, `CopyPaths`, `CopyAbsolutePath`, `CopyReference`, `Exit`, all `FileChooser.*` rows (section 6) |
| Covered in 06 | `CloseAllProjects`, `CloseOtherProjects` |
| LATER (platform tooling) | `CreateDesktopEntry` ("Create Desktop Entry..."), `CreateLauncherScript` ("Create Command Line Launcher..."), `DeleteOldAppDirs` ("Delete Leftover IDE Directories..."), `DeleteRecentFilesFallback`, `ExportToEclipse`, `ExportToHTML`, `ExportToTextFile`, `Print`, `SaveAsNewFormat` ("Save as Directory-Based Format..."), `LightEditOpenFileInProject`, `WslSync`, `WelcomeFileChangeLanguage` (welcome project), `FileChooser.GoToWslHome`, `FileChooser.LightEditGotoOpenedFile` |

## 8. Open questions

1. The default `ide.scratch.buffers` count and the `buffer` file naming were not read (assumed `buffer1.txt`...).
2. The exact order and grouping of the language popup of "New Scratch File" (recent languages first; list of all languages with LRU persistence key) needs a reference run.
3. Template engine: the reference uses a Velocity-compatible engine; Archipel must define its subset (variables above, `#parse`, `#if`, `#else`, `#end`, `#foreach`, `#set`); the exact list of predefined variables and directives needs a decision and a reference comparison.
4. Save As default directory and extension handling.
5. Text of the "Invalidate Caches" dialog options (only the description message was read).
6. Line-separator menu labels (see `04-status-bar.md` open question 3).
7. How a file with inconsistent line separators is reported (silent normalization vs. a notification).
8. Layout of the "Save File as Template" dialog (fields were not read; name and extension are assumed).

# Workbench spec 05: Settings dialog, settings pages, Keymap

Status: specification (clean-room). ID prefixes `WB-SET` (dialog behavior and page catalog) and `WB-KEY` (keymap page, keymaps, shortcut model).
Priorities `MUST` / `SHOULD` / `LATER` as in `docs/specs/README.md`. Executable cases: `cases/05-settings-dialog-and-keymap.cases.md`.

Scope: how the Settings dialog opens, searches, applies and cancels; project vs IDE level pages; the complete list of settings pages the inventory registers (name, group,
parent, scope, purpose, defaults where read, priority); the Keymap page (selector, search, shortcut editing, conflicts, copies); the 26 bundled keymaps with their
parents and the differences that matter. The contents of pages that belong to other areas (Inspections, Code Style, Plugins, VCS, build) are owned by those specs; here only
their registration (id, name, placement) is specified.

Sources studied: `ic: platform/platform-impl/src/com/intellij/openapi/options/newEditor/` (`SettingsDialog.java`, `SettingsEditor.java`, `ConfigurableEditor.java`, `SettingsTreeView.java`,
`SettingsFilter.kt`), `ic: platform/platform-impl/src/com/intellij/openapi/keymap/impl/` (`DefaultKeymap.kt`, `KeymapManagerImpl.kt`, `KeymapImpl.kt`, `ui/KeymapPanel.java`),
`ic: platform/platform-api/resources/messages/KeyMapBundle.properties`, `ic: platform/ide-core/resources/messages/OptionsBundle.properties`, `ic: platform/platform-api/resources/messages/IdeBundle.properties`,
`UIBundle.properties`, `ic: platform/platform-resources/src/keymaps/*.xml`, `ic: plugins/keymaps/*/resources/keymaps/*.xml`, `ic: platform/editor-ui-api/src/com/intellij/ide/ui/UISettingsState.kt`, inventory `settings-keymap.md`.

## 1. The Settings dialog

Open: **Ctrl+Alt+S** (`ShowSettings`; macOS Cmd+, and Cmd+;), File | Settings, Welcome screen Configure/Options, the settings entry-point gear. Title is the page name when the
dialog was opened for a single page, otherwise **"Settings"**. Modal over the IDE, resizable, minimum size 900 x 700 px; the content area prefers 800 x 600 px; size and the splitter
proportion between tree and page persist. Dialog buttons, left to right: **OK**, **Cancel**, **Apply**, Help (`?`) when the page has a help topic. There is **no Reset button** in the main dialog.

Layout: left column = search field above a **tree of pages**; right column = **page header** (breadcrumb path of the current page, for example `Editor > General > Appearance`, plus **Back** and
**Forward** history buttons) above the page content; an error band at the bottom of the page appears when Apply failed for the current page.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-SET-001 | MUST | Ctrl+Alt+S opens the dialog on the last selected page (persisted), or the first page on first use. With a filter text argument (from "Search Everywhere" results) it opens with that filter applied. | Open, select Keymap, OK; reopen: Keymap selected. |
| WB-SET-002 | MUST | OK = apply all modified pages, then close; Cancel = cancel all modified pages (each page's `cancel` then `reset`) and close **without a confirmation dialog**; Apply = apply all modified pages and keep the dialog open. Apply is **enabled only while some page is modified**. | Change a checkbox: Apply enabled; click Apply: disabled again, change persisted; Cancel after another change: change discarded, no prompt. |
| WB-SET-003 | MUST | Apply visits modified pages in tree order. Each page's apply may fail with a **configuration error** (title and message). All failing pages are marked; the dialog stays open, selects the **first failing page** (or the page named as the error's originator) and shows the message in the red error band below the page, or as an error message dialog if the band is unavailable. Pages that applied successfully are no longer modified. | Page A applies, page B throws "Invalid port": dialog stays; B selected; error band "Invalid port"; A saved. |
| WB-SET-004 | MUST | **Modified marker:** in the tree, the name of a page with unsaved changes is drawn in the modified color (theme key `Tree.modifiedItemForeground`, default blue); a page with an apply error is drawn in the error color (`Tree.errorForeground`, default red). Groups are drawn bold. | Change a checkbox on "Keymap"... tree entry turns blue; after Apply it returns to normal. |
| WB-SET-005 | MUST | Switching to another page keeps unsaved changes of the left page (they remain pending and the page stays marked modified) unless the page's "leave state" says it was not touched by the user. | Edit page A, select page B, return to A: edit still present. |
| WB-SET-006 | MUST | The header shows a link-like action **"Revert changes"** (tooltip "Revert all changes on this page to the previously saved state") enabled only while the current page is modified; it re-reads saved values into the page. The aggregated action "Revert All" ("Revert all settings to the previously saved state") resets all modified pages (used by non-modal settings). | Change a field, click Revert changes: field back to the saved value, page unmodified. |
| WB-SET-007 | MUST | **Escape** closes the dialog (as Cancel) unless the search field holds text: then Escape **clears the search first** and keeps the dialog open. Enter in the search field selects the current page and moves focus into the page. | Type "font" in search, Escape: field empty, dialog open; Escape again: dialog closes. |
| WB-SET-008 | MUST | Ctrl+F (the Find action) in the dialog focuses the search field. | Press Ctrl+F: caret in search. |
| WB-SET-009 | SHOULD | Back / Forward (header toolbar) navigate the history of selected pages (Alt+Left / Alt+Right as the Back/Forward actions). | Select Appearance, then Keymap, Back: Appearance. |
| WB-SET-010 | MUST | Settings are saved when the dialog applies (written to the config store immediately after apply with an explicit save of all settings), not at IDE exit. | Apply, kill the process: values persisted. |
| WB-SET-011 | SHOULD | The "Settings" tool window (id `Settings`, anchor left) is the non-modal variant: the same tree and page without OK/Cancel; changes are applied by Apply; leaving a page auto-resets pages that were not edited. | Open via the entry-point menu option: page shows in the tool window. |

### 1.1 Search

The search field filters the tree to pages **whose name or whose option labels/keywords match** the text. Matching is case-insensitive, word-prefix and camel-hump friendly (for example `ws` matches
"Whitespaces"). While filtering, the tree expands to show the matching pages, the best matching page is selected, and inside the page options that match are **highlighted** (a *spotlight*:
the rest of the page is dimmed while focus is in the tree or the search field; the spotlight is removed when focus moves into the page).
The filter runs after the typing pauses (alarm of a few hundred ms); with no match the tree is empty and the page shows the "Nothing found" empty state.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-SET-020 | MUST | Search matches the page name and the searchable option texts of every page (including pages not yet created). | Search "tab": pages "Editor Tabs" (under Editor > General) and others with a tab option appear. |
| WB-SET-021 | MUST | The first match is selected automatically; Up/Down in the search field move the tree selection. | Type "keym": "Keymap" selected. |
| WB-SET-022 | MUST | Clearing the text restores the full tree and keeps the current page selected. | Clear the field: all groups shown; same page selected. |
| WB-SET-023 | SHOULD | The spotlight dims everything except matching controls while focus is in the search field or tree. | Search "line numbers": that checkbox looks highlighted; the rest dimmed. |

## 2. Project vs IDE level

Every page has a **scope**: `application` (IDE-wide, stored in the global config) or `project` (stored with the project, per project). The tree has top groups; project pages carry a "For current project"
marker in the reference UI by being in the same tree but their values are stored in the project. The Default Project dialog (opened from the Welcome screen) shows group "Default Project" and the hint "These settings will be applied to new projects" is the dialog's south hint (`template.settings.hint`).

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-SET-030 | MUST | Page scope is a property of the page: project-scope pages read and write the project's settings (per-project `.archipel` store), application-scope pages the global store; one dialog edits both. Apply writes each to its store. | Change "File Encodings" (project) in project A: project B unchanged. Change "Keymap" (application): applies everywhere. |
| WB-SET-031 | MUST | With no open project (Welcome screen) only application pages and the Default Project group are available. | Welcome > Configure > Settings: tree shows application groups; project pages are in "Default Project". |
| WB-SET-032 | SHOULD | Project-scope pages in the catalog are marked in the tooltip of the tree entry ("For current project"). | n/a (visual). |

## 3. Tree structure (groups)

Top-level groups (display names, in this order): **Appearance & Behavior** (id `appearance`), **Keymap**, **Editor**, **Plugins**, **Version Control**, **Build, Execution, Deployment**, **Languages & Frameworks**,
**Tools**, **Advanced Settings**, then **Other Settings** (plugin pages without a group), **AI**, **Profilers**, **Natural Languages** when plugins supply pages. Group captions are bold. Child pages are indented under their parent;
the placement for each inventory page is in section 4.

## 4. Page catalog (inventory `settings-keymap.md`, `todo.md`, `problems-analyze.md`, `project-view.md`)

Legend: Pri = priority for the first Archipel workbench release (MUST: the page exists with the listed behavior; SHOULD: page exists with its main options; LATER: registered as a placeholder entry or omitted). "Defaults" lists the
reference values that were read; `see X` points to the area that specifies contents.

| Id | Display name | Placement (group / parent) | Scope | Purpose | Defaults read from the reference | Pri |
|---|---|---|---|---|---|---|
| `preferences.lookFeel` | Appearance | Appearance & Behavior | app | Theme, IDE font and zoom, tool window and window options | Show tool window bars on (`hideToolStripes` off); Show tool window names off; Show tool window numbers off; Widescreen layout off; Side-by-side layouts left/right off; Remember size for each tool window: Old UI on, New UI off; status bar on; main menu on; navigation bar on; editor tab limit 30; show file icon in tabs on | MUST |
| `preferences.customizations` | Menus and Toolbars | Appearance & Behavior | app | Customize menus, toolbars, popups | none (empty customization) | LATER |
| `preferences.general` | System Settings | Appearance & Behavior | project | Startup, project opening, file saving, external sync, deletion, running-process close behavior | "When closing a tool window with a running process": Terminate process / Disconnect / **Ask**; autosave options: see `platform/09-saving-and-autosave.md`; "Sync external changes" options; "Move files to the bin instead of deleting permanently" | MUST |
| `application.passwordSafe` | Passwords | child of System Settings | app | Where credentials are stored | n/a | LATER |
| `http.proxy` | HTTP Proxy | child of System Settings | app | Proxy for outgoing connections | no proxy | SHOULD |
| `consents` | Data Sharing | child of System Settings | app | Usage-statistics and consent settings | off until the user consents | LATER (Archipel sends no data by default) |
| `ide.date.format` | Date Formats | child of System Settings | app | Date and time formats | system locale | LATER |
| `preferences.language.and.region` | Language and Region | child of System Settings | app | UI language, region | English | LATER |
| `http.certificates` | Server Certificates | child of System Settings | app | Accepted TLS certificates; checkbox "Accept non-trusted certificates automatically" | off | LATER |
| `preferences.updates` | Updates | child of System Settings | app | Update channel and check | n/a | LATER |
| `reference.settings.ide.settings.notifications` | Notifications | Appearance & Behavior | app | Per-group notification display; see `07-notifications-and-background-tasks.md` | "Display balloon notifications" on; location default | MUST |
| `preferences.accessibility` | Accessibility | Appearance & Behavior | app | Screen reader support, contrast scrollbars | "Support screen readers" off; "Use contrast scrollbars" off | SHOULD |
| `reference.idesettings.quicklists` | Quick Lists | Appearance & Behavior | app | Named lists of actions for the keymap | no quick lists ("No quick lists") | LATER |
| `reference.settings.ide.settings.file-colors` | File Colors | Appearance & Behavior | project | Background colors per scope (tabs, project view) | none | LATER |
| `preferences.pathVariables` | Path Variables | Appearance & Behavior | project | Path macros used in settings | none | LATER |
| `preferences.externalDependencies` | Required Plugins | Appearance & Behavior | project | Plugins required to open this project | none | LATER |
| `trusted.hosts` | Trusted Locations | Appearance & Behavior | app | Directories trusted as project sources; label "Projects and files located under these local directories will be considered as trusted"; add dialog "New Trusted Location"; file chooser "Select Trusted Location" | empty list | MUST (see `06-welcome-new-project-open.md`) |
| `com.intellij.platform.ide.impl.presentationAssistant.PresentationAssistantConfigurable` | Presentation Assistant | Appearance & Behavior | app | Shows pressed shortcuts as an overlay | off | LATER |
| `preferences.keymap` | Keymap | root group | app | The keymap page (section 6) | active keymap = OS default (section 7) | MUST |
| `preferences.editor` | General (editor) | Editor | app | Editor behavior: soft wraps, whitespace display, scrolling, mouse, breadcrumbs entries | see `docs/specs/codeinsight` and editor specs | MUST (name and placement; contents by editor spec) |
| `preferences.editor.code.editing` | Code Editing | Editor | app | Completion, hints, intention bulb, quick documentation | see codeinsight specs | MUST (placement) |
| `Console` | Console | Editor > General | app | Console buffer size, wrapping | n/a | LATER |
| `editor.breadcrumbs` | Breadcrumbs | Editor > General | app | Breadcrumb bar options | see `PLAT-LANG-088` | SHOULD |
| `editor.stickyLines` | Sticky Lines | Editor > General | app | "Show sticky lines while scrolling", "Maximum number of lines" | feature on | SHOULD |
| `editor.preferences.fonts.default` | Font | Editor | app | Editor font family, size, line height, ligatures | n/a (editor spec) | MUST (placement) |
| `app.console.font` | Console Font | Editor > Font | app | Console font | inherits editor font | LATER |
| `reference.settingsdialog.IDE.editor.colors` | Color Scheme | Editor | app | Editor color scheme (language defaults, console colors, diff colors) | scheme follows theme | SHOULD |
| `preferences.fileTypes` | File Types | Editor | app | File type to pattern associations and ignored files | see `platform/01-vfs-and-documents.md` | SHOULD |
| `File.Encoding` | File Encodings | Editor | project | Global encoding, project encoding, per-path encoding table (columns "Path", "Encoding"), "Create UTF-8 files:" with BOM policy (`with BOM`, `with NO BOM`, `with BOM under Windows, with no BOM otherwise`) | policy "with NO BOM" | MUST |
| `editor.reader.mode` | Reader Mode | Editor | project | Read-only reading presentation | off | LATER |
| `com.intellij.minimap` | Minimap | Editor | app | Code minimap on the editor edge | off by default in tabs menu entry "Show Minimap" | LATER |
| `preferences.toDoOptions` | TODO | Editor | app | TODO patterns and filters; see `03-structure-problems-todo.md` | patterns `\btodo\b.*`, `\bfixme\b.*` | SHOULD |
| `Errors` | Inspections | Editor | project | Inspection profiles; see `docs/specs/codeinsight/03-inspections.md` | profile Default | MUST (placement) |
| `project.scopes` | Scopes | Appearance & Behavior | project | Named scopes; see `02-project-view.md` WB-PV-003 | predefined scopes only | SHOULD |
| `preferences.pluginManager` | Plugins | root group | app | Plugin manager | n/a | MUST (Archipel plugin host entry) |
| `advanced.settings` | Advanced Settings | root group | app | Rarely changed flags, searchable, grouped by area | each flag at its documented default (for example "Show only line bookmarks in popup", "Enable Scratches and Consoles in the Project view") | SHOULD |
| `settings.sync` | Backup and Sync | root group | app | Cloud settings sync | off | LATER |
| `ics` | Settings Repository | Tools | app | Git-based settings sync | none | LATER |
| `reference.settings.ide.settings.web.browsers` | Web Browsers and Preview | Tools | app | Browser list, reload behavior | system default browser | LATER |
| `actions.on.save` | Actions on Save | Tools | project | Reformat, optimize imports, run inspections on save; see CI-OPT-043 | see CI-OPT | SHOULD |
| `lsp.servers` | Language Servers | Languages & Frameworks | project | Language server list | none | LATER |
| `tasks` | Tasks | Tools | project | Issue tracker integration | none | LATER |
| `lombok` | Lombok | Languages & Frameworks | project | Lombok plugin options | plugin | LATER |

Settings-related actions (inventory): `ShowSettings` ("Settings...", Ctrl+Alt+S), `ShowProjectStructureSettings` ("Project Structure...", Ctrl+Alt+Shift+S; LATER), `QuickChangeScheme` ("Quick Switch Scheme...", Ctrl+` , see 5), `ChangeLaf` ("Theme"), `ChangeColorScheme`
("Editor Color Scheme"), `ChangeKeymap` ("Keymap"), `ChangeView` ("View Mode"), `ChangeToolWindowLayout` ("Layout"), `ChangeIdeScale` ("Zoom"), `ChangeInspectionProfile` ("Error Highlighting"), `ExportSettings`, `ImportSettings`,
`RestoreDefaultSettings` ("Restore Default Settings..."), `ConfigureIcs`, `MergeSettings`, `ResetToMySettings`, `ResetToTheirsSettings`, the Backup-and-Sync actions (`SettingsSyncOpenSettingsAction`, `settingsSync.*`), `CustomizeMainToolbarAction`
("Customize Toolbar..."), `CustomizeUI`, `ShowSettingsChangesLogFile`, `ShowSettingsWithAddedPattern`, `Settings.Paste` ("Paste" in settings text fields), `internal.settings.v2.DumpSettingsContext`: all LATER except `ShowSettings`, `QuickChangeScheme`, `ChangeKeymap`, `ChangeLaf`, `ChangeColorScheme` (SHOULD).

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-SET-040 | MUST | The pages marked MUST in the catalog are registered with exactly these ids, names, groups and scopes; the dialog shows them in the tree in the group order of section 3. | A test lists the tree and compares names and nesting. |
| WB-SET-041 | MUST | Every registered page has searchable keywords so that Search Everywhere's "Actions"/"Settings" can find it by name. | Search Everywhere "keymap": Settings result "Keymap". |
| WB-SET-042 | SHOULD | LATER pages may be absent; absent pages never break links from other areas (a link to a missing page shows "Settings page not found"). | Open `lombok` link: message "Settings page not found". |
| WB-SET-043 | MUST | **Quick Switch Scheme** (Ctrl+`) opens a popup with the entries Theme, Editor Color Scheme, Keymap, Code Style Scheme, Error Highlighting, View Mode, Layout, Zoom (each opens a submenu of schemes); selecting a scheme applies it at once. | Ctrl+`, Keymap, Eclipse: the active keymap becomes Eclipse. |

---

## 5. Keymap page

Page id `preferences.keymap`, placed as its own top group "Keymap". Layout, top to bottom:

1. **Keymap selector** (combo): the active keymap's presentable name; the list shows all visible keymaps, bundled first ordered by name with the OS default first, then user keymaps under their parent. A label "Based on {parent} keymap" appears for derived keymaps. A gear menu next to it: Duplicate, Rename, Delete (user keymaps only), Reset (restore bundled keymap's own shortcuts when copied).
2. **Toolbar**: Search field (filters the action tree by action name), the button **Find Actions by Shortcut** (popup "Find Shortcut" with a key-press field, a checkbox "Second stroke" and a mouse pad "Mouse shortcut"), **Clear Filtering**, Expand All / Collapse All, and "Show conflicts with system shortcuts" (system shortcut toggle).
3. **Action tree** with columns **Action** and **Shortcuts**; the Shortcuts cell lists the shortcuts as text, with `or {n} more` when too long. Top-level groups: Main Menu, Main Toolbar, Editor Popup Menu, Editor Tab Popup Menu, Project View Popup Menu, Editor Gutter Popup Menu, Scope View Popup Menu, Navigation Bar Popup Menu / Toolbar, Debugger Actions, Editor Actions, Tool Windows, External Tools, Version Control Systems, Intentions, Macros, Quick Lists, Plugins, Other. An action without a shortcut has an empty Shortcuts cell (accessible text `<no shortcut>`).
4. Option checkboxes (platform dependent): "Enable mnemonics in menu", "Enable mnemonics in controls", "Use national layouts for shortcuts (requires restart)", "Use US non-alphanumeric keys for shortcuts (requires restart)", "Show F1, F2, etc. keys on the Touch Bar (requires restart)" (macOS), and a link "Get more keymaps in {Settings} | Plugins".

### 5.1 Requirements

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-KEY-001 | MUST | The page lists all visible keymaps (section 7), the active one is selected; choosing another keymap applies on Apply/OK and takes effect immediately in menus and key handling. | Select Eclipse, OK: Go to File is Ctrl+Shift+R. |
| WB-KEY-002 | MUST | **Editing a bundled (read-only) keymap creates a copy first.** The first change (add, remove or reset a shortcut) on a keymap that cannot be modified creates a user keymap named `{name} copy` (if taken, `{name} copy {n}`; numbering: open question 2), whose parent is the bundled keymap, selects it as active in the page, and applies the change to the copy. The bundled keymap stays untouched. | Edit "Default": the selector shows "Windows copy" (name of the OS default's presentable name plus " copy") and the shortcut is in it. |
| WB-KEY-003 | MUST | User keymaps store only **differences** from the parent: shortcuts added, and "unbind" markers for removed parent shortcuts. Effective shortcuts of an action = parent's unless the child overrides the action; an override with no shortcut unbinds. | In "Windows copy" remove Ctrl+C from Copy: Copy has no shortcut in the copy; the parent still has it. |
| WB-KEY-004 | MUST | Right click on an action (or Edit shortcut) offers: **Add Keyboard Shortcut**, **Add Mouse Shortcut**, **Add Abbreviation** (for actions that support abbreviations in Search Everywhere), and for each existing shortcut **Remove {shortcut}**; plus **Remove All Shortcuts** and **Reset Shortcuts** (restore the parent's bindings for that action). The popup title is "Edit Shortcuts". | Right click Rename: menu includes "Add Keyboard Shortcut", "Add Mouse Shortcut", "Add Abbreviation", "Remove Shift+F6", "Remove All Shortcuts", "Reset Shortcuts". |
| WB-KEY-005 | MUST | **Add Keyboard Shortcut** opens "Keyboard Shortcut" with a key-capture field ("press keys"), a checkbox "Second stroke:" enabling a second capture field, and below the list "Already assigned to:" showing every action (in this keymap) that has the same shortcut, updated live; OK/Cancel. Esc inside the field clears; modifiers alone are not a shortcut. | Press Ctrl+Alt+L: the list shows "Reformat Code" (in the default keymap). |
| WB-KEY-006 | MUST | **Conflict on OK**: if the shortcut is assigned to other actions in the keymap, the dialog "Warning" asks "This shortcut is already assigned to other actions. Do you want to remove the other assignments?" with **Remove**, **Keep**, **Cancel**. Remove: the other assignments are removed from this keymap (a copy if bundled) and the shortcut is added; Keep: added alongside; Cancel: nothing. A single stroke that is the first stroke of an existing chord is a conflict too (open question 6). | Assign Ctrl+Alt+L to "Find Usages" and press Remove: Reformat Code loses Ctrl+Alt+L. |
| WB-KEY-007 | MUST | **Search by action name** in the page's search field filters the tree to matching actions (and their groups), expanding them; the match is case-insensitive substring/word match over the action's name and its group path. | Search "reformat": "Reformat Code" and "Reformat File..." shown. |
| WB-KEY-008 | MUST | **Search by shortcut** (Find Actions by Shortcut): pressing keys in the popup filters the tree to actions that have that shortcut as first stroke (or exactly, when "Second stroke" is on and both strokes pressed); "Clear Filtering" restores the tree. A mouse pad accepts a click/double-click/wheel shortcut with modifiers. | Press Ctrl+S: tree shows "Save All". |
| WB-KEY-009 | MUST | **Mouse shortcuts**: dialog "Mouse Shortcut" with the pad text "Enter a shortcut here: single or double-click, scroll the wheel, modify with Ctrl, Alt and Shift"; the shortcut text is `Ctrl+Click`, `Double Click`, `Middle-Click`, `Right-Click`, `Button 4 Click`, `Button 5 Click`, `Wheel up`, `Wheel down` (with modifiers prefixed: `Ctrl+Shift+Click`). | Click with Ctrl on the pad: text "Ctrl+Click". |
| WB-KEY-010 | SHOULD | **Abbreviations**: "Add Abbreviation" asks "Enter new abbreviation:" (dialog "Abbreviation"); abbreviations are shown in the Shortcuts column and removable with "Remove Abbreviation '{x}'". | Add abbreviation "cmt" to "Comment with Line Comment": Search Everywhere "cmt" finds it. |
| WB-KEY-011 | MUST | User keymap management: duplicate (`{name} copy`), rename, delete (not for bundled keymaps; a user keymap that is a parent of another cannot be deleted without reparenting) and reset. Names must be unique and non-empty: errors "All keymaps should have non empty names" and "All keymaps should have unique names". | Rename two keymaps to the same name: error "All keymaps should have unique names"; Apply refused. |
| WB-KEY-012 | SHOULD | The tree supports the actions registered by tool windows (every tool window's `Activate<Id>ToolWindow` action under group "Tool Windows"), macros, external tools and quick lists, and plugin actions. | Group "Tool Windows" lists "Project", "Bookmarks", ... with Alt+digit shortcuts. |
| WB-KEY-013 | MUST | Keymap changes apply at once after OK/Apply to menu accelerators and the key dispatcher without restart (national layout options excepted). | Remap Rename to F2, OK: Refactor menu shows F2; pressing F2 renames. |
| WB-KEY-014 | SHOULD | "Show conflicts with system shortcuts" (Linux/macOS desktops) marks shortcuts that the OS reserves; the macOS keymap warns "{x} and {y} shortcuts conflict with the macOS system shortcuts." | n/a (desktop dependent). |
| WB-KEY-015 | MUST | A keymap that **disables mnemonics** (`disable-mnemonics`, true for the macOS keymaps) makes Alt+letter menu mnemonics inert when its owner checkbox says so. | Mac keymap: Alt+F does not open the File menu. |
| WB-KEY-016 | LATER | "Proper Redo" / "Proper insert inline proposal" variants (`{name} Proper Redo`), Ctrl+Y migration dialog "Map Action to Ctrl+Y" (Redo / Delete Line / Cancel). | n/a |

## 6. Shortcut model and dispatch rules

* A **keyboard shortcut** = first keystroke, optional second keystroke (a **chord**), each = modifier set + key. Text form in the UI: `Ctrl+Alt+S`, `Ctrl+K, Ctrl+C`, `Shift+Shift` (a double-modifier **gesture**: `Shift Shift` = Search Everywhere, `Ctrl Ctrl` held = Run Anything).
* Key names in the keymap files (reference): `control`, `alt`, `shift`, `meta`; special keys uppercase (`ESCAPE`, `F11`, `INSERT`, `BACK_QUOTE`, `QUOTE`, `OPEN_BRACKET`, `ADD`, `SUBTRACT`, `MULTIPLY`, `SLASH`, `PERIOD`, `COMMA`, ...).
* Resolution: the active keymap is searched; if the keymap has no entry for an action id, its parent is searched (recursively to `$default`); a child entry for an action id **replaces** the parent's list for that action; an entry with no shortcuts **unbinds**.
* **Mouse shortcuts** use the same inheritance.
* When several actions share a shortcut in the keymap, the dispatcher picks the **enabled** action of the highest-priority context (focused component, then editor, then global); if several remain, a popup lets the user choose ("shortcut conflict" chooser).

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-KEY-020 | MUST | Chord shortcuts: after the first stroke the status bar shows "Prefix key pressed" hint; the second stroke completes; any non-matching key cancels the chord; Escape cancels. | VS keymap: Ctrl+K pressed: status text "Prefix key pressed"; K completes Toggle Bookmark. |
| WB-KEY-021 | MUST | Inheritance and unbinding as in the model above. | Keymap `Windows copy` with ShowSettings overridden to nothing: Ctrl+Alt+S does nothing. |
| WB-KEY-022 | SHOULD | The gesture shortcuts `Shift Shift` (Search Everywhere) and `Ctrl Ctrl` (Run Anything, double-press-and-hold) are keymap entries (`keyboard-gesture-shortcut`, modifier `dblClick`). | Press Shift twice quickly: Search Everywhere opens. |

---

## 7. Bundled keymaps

The OS default (first in the list): **Windows** (`$default`) on Windows; **macOS** (`Mac OS X 10.5+`) on macOS; **GNOME** on GNOME, **KDE** on KDE, **XWin** on other Linux/Unix desktops. Presentable names: `$default` -> "Windows",
`Mac OS X 10.5+` -> "macOS", `Default for GNOME` -> "GNOME", `Default for KDE` -> "KDE", `Default for XWin` -> "XWin", `Mac OS X` -> "IntelliJ IDEA Classic" (suffix " (macOS)" off macOS). Linux-specific keymaps are **hidden** on Windows and macOS;
macOS keymaps are hidden on other systems. Shipping all of them is `LATER`; the MUST set is the OS default plus **Windows**, **macOS**, **GNOME/XWin/KDE** for the matching desktop.

Inheritance ("parent") and a differences summary. Shortcuts in the last columns are first shortcuts for: Go to Action (`GotoAction`), Go to File (`GotoFile`), Rename (`RenameElement`), Activate Project (`ActivateProjectToolWindow`); `-` = inherited unchanged from the parent; in the default keymap these are Ctrl+Shift+A, Ctrl+Shift+N, Shift+F6, Alt+1.

| Keymap (file) | Parent | Pri | Own entries | Differences at a high level |
|---|---|---|---|---|
| Windows (`$default.xml`) | none | MUST | 422 | the base: Alt+digit tool windows, Ctrl+Shift+A, double-Shift Search Everywhere, F11 bookmarks |
| XWin (`Default for XWin.xml`) | $default | MUST (Linux) | 16 | avoids Alt+digit clashes: Run to Cursor Shift+Alt+9, Quick Evaluate Ctrl+Alt+8, Show Usages Ctrl+Alt+7, Zoom IDE Shift+Ctrl+Alt+=/-, File Path Ctrl+Alt+Shift+2 |
| GNOME (`Default for GNOME.xml`) | XWin | MUST (GNOME) | 15 | Back/Forward Shift+Alt+Left/Right, Find Usages Shift+Alt+7, Find also Shift+Alt+3, Select In Shift+Alt+1 (and Alt+F1), Next/Previous Occurrence Shift+Alt+Down/Up; no default Move Line Up/Down |
| KDE (`Default for KDE.xml`) | XWin | MUST (KDE) | 48 | uses Ctrl+digit for many actions: Close Tab Ctrl+4, Refresh Ctrl+5, Stop Ctrl+2, File Structure Ctrl+0, Find Usages in File Ctrl+7; unbinds Ctrl+digit bookmarks; Reformat Shift+Alt+L, Introduce Variable Shift+Alt+V |
| macOS (`Mac OS X 10.5+.xml`) | $default | MUST (macOS) | 240 | Cmd+digit tool windows, Cmd+, Settings, Cmd+; Project Structure, Cmd+N New, Cmd+Shift+N Scratch, Cmd+R Refresh, F3 bookmark; Go to File Cmd+Shift+O; Back Cmd+[; mnemonics disabled |
| IntelliJ IDEA Classic (`Mac OS X.xml`) | $default | LATER | 168 | older macOS mapping with Cmd+digit; mnemonics disabled |
| macOS System Shortcuts (`macOS System Shortcuts.xml`) | macOS | LATER | 102 | adds Cmd+Shift+P Go to Action, Cmd+Alt+R Rename, Cmd+Shift+A, Ctrl+Shift+K bookmark |
| Emacs | $default | LATER | 115 | Go to Action `Esc X` / `Alt+X`, Go to File `Ctrl+X Ctrl+F`, Find `Ctrl+S`, Alt+F3 |
| Sublime Text | $default | LATER | 121 | Go to Action `Ctrl+Shift+P`, Go to File `Ctrl+P`, Find `Ctrl+F`, Toggle Bookmark `Ctrl+F2` |
| Sublime Text (Mac OS X) | macOS | LATER | 138 | same with Cmd (`Cmd+Shift+P`, `Cmd+P`) |
| Eclipse | $default | LATER | 162 | Go to File Ctrl+Shift+R, Rename Shift+Alt+R, Toggle Bookmark Ctrl+Shift+F11; many Eclipse debug keys |
| Eclipse (Mac OS X) | macOS | LATER | 168 | Go to Action Cmd+3, Go to File Cmd+Shift+R |
| NetBeans 6.5 | $default | LATER | 170 | Go to Action Ctrl+I, Go to File Shift+Alt+O, Rename Ctrl+R, Activate Project Ctrl+1 |
| QtCreator / QtCreator (Mac OS X) | $default / macOS | LATER | 166 / 186 | Rename Ctrl+Shift+R, Activate Project Alt+X, Toggle Bookmark Ctrl+M |
| ReSharper / ReSharper OSX | Visual Studio / Visual Studio OSX | LATER | 75 / 69 | Rename F2, Go to File Ctrl+Shift+N, Alt+digit tool windows |
| Visual Assist / Visual Assist OSX | Visual Studio / Visual Studio OSX | LATER | 21 / 10 | few overrides (Go to File Shift+Alt+O, Rename Shift+Alt+R) |
| Visual Studio / Visual Studio OSX | $default / macOS | LATER | 200 / 173 | chords: Rename `Ctrl+R, R`, Toggle Bookmark `Ctrl+K, K`; Go to File Ctrl+Shift+T |
| Visual Studio 2022 | Visual Studio | LATER | 69 | Go to Action Ctrl+Q, Rename F2 or `Ctrl+R, R` |
| Visual Studio for Mac | macOS | LATER | 82 | Go to File Cmd+P, Rename Cmd+R / F2 |
| VSCode / VSCode OSX | $default / macOS | LATER | 300 / 316 | Go to Action Ctrl+Shift+P (Cmd+Shift+P, F1), Go to File Ctrl+P, Rename F2, Project Ctrl+Shift+E |
| Xcode | macOS | LATER | 206 | Go to File Cmd+O, Rename Ctrl+Cmd+E, Toggle Bookmark Ctrl+F3 |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-KEY-030 | MUST | The default keymap per OS is chosen as above on first start and persists as a user choice. | Linux without GNOME/KDE: "XWin" selected on first start. |
| WB-KEY-031 | MUST | Archipel ships the base keymap `Windows`, the macOS keymap, and the three Linux variants with the entries of the table (differences from their parent). Parent links are by name. | Dump of XWin shows exactly the 16 overrides of the table's first row group. |
| WB-KEY-032 | LATER | The other bundled keymaps and their Mac variants, with their parent relations as in the table. | n/a |
| WB-KEY-033 | MUST | Keymaps hidden for the platform are not listed (Linux keymaps on Windows/macOS; macOS keymaps off macOS). | Windows: the selector has no "GNOME". |

## 8. Open questions

1. Search Everywhere settings integration detail (ranking of page hits) is owned by the search-everywhere area.
2. Exact copy-name algorithm for `{name} copy {n}` when "copy" exists (the reference has the two message forms `{0} copy` and `{0} copy {1}`; the start index of n needs a reference run).
3. Exact grouping order and captions of the Keymap action tree beyond the bundle keys listed (Main Menu, Main Toolbar, ..., Other).
4. The "no shortcut" rendering of empty cells in the action tree; tooltip wording.
5. Default values for several System Settings options (confirm exit, restore projects on startup, delete to bin) were not read from the source; they need a reference run before implementation.
6. Whether a one-stroke shortcut that is only the prefix of a chord is reported in the "Already assigned to" list (assumed yes).

# File operations (save, sync, reload, scratch files, VFS)

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (56)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `AddBom` | Add BOM | `AddBom.Group`, `EncodingPanelActions` |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `AssociateWithFileType` | Associate with File Type… | `CommanderPopupMenu`, `ProjectViewPopupMenu`, `NavbarPopupMenu` (+2) |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/lang-impl` |
| `ChangeFileEncodingAction` | File Encoding | `FilePropertiesGroup` |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `CloseAllProjects` | Close All Projects | `FileOpenGroup` |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `CloseOtherProjects` | Close Other Projects | `FileOpenGroup` |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `CopyAbsolutePath` | Absolute Path | `CopyFileReference` | as `CopyPaths` | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/lang-impl` |
| `CopyPaths` | Copy Paths | `CutCopyPasteGroup`, `EditorTabPopupMenu` | `control shift C` | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `CopyReference` | Copy Reference | `CopyReferencePopupGroup`, `EditorTabPopupMenu`, `Copy.Paste.Special` | `control alt shift C` | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/lang-impl` |
| `CreateDesktopEntry` | Create Desktop Entry… | `ToolsMenu`, `WelcomeScreen.Configure`, `WelcomeScreen.Options` |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `CreateLauncherScript` | Create Command Line Launcher… | `ToolsMenu` |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `DeleteOldAppDirs` | Delete Leftover IDE Directories… | `HelpMenu` |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `DeleteRecentFilesFallback` | Delete from Recent Files | `SwitcherAndRecentFilesFallback` | as `$Delete` | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-api` |
| `Exit` | Exit | `FileMenu` |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `ExportToEclipse` | Export Project to Eclipse… | `FileExportGroup` |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `plugins/eclipse` |
| `ExportToHTML` | Export Files or Selection to HTML… | `FileExportGroup` |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/lang-impl` |
| `ExportToTextFile` | Export to Text File | `UsageView.Popup` | `alt O` | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `FileChooser.Delete` | Delete… | `FileChooserToolbar` | as `$Delete` | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `FileChooser.GoBackward` | Previous Directory | `FileChooserToolbar` | as `Back` | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `FileChooser.GoForward` | Next Directory | `FileChooserToolbar` | as `Forward` | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `FileChooser.GoToParent` | Parent Directory | `FileChooserToolbar` | `BACK_SPACE` | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `FileChooser.GoToRoot` | Root Directory | `FileChooserToolbar` | `control BACK_SLASH` | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `FileChooser.GoToWslHome` | Home Directory in WSL | `FileChooserToolbar` |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `FileChooser.GotoDesktop` | Desktop Directory | `FileChooserToolbar` | `control D` | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `FileChooser.GotoHome` | Home Directory | `FileChooserToolbar` | `control 1` | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `FileChooser.GotoModule` | Module Directory | `FileChooserToolbar` | `control 3` | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `java/java-backend` |
| `FileChooser.GotoProject` | Project Directory | `FileChooserToolbar` | `control 2` | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `FileChooser.LightEditGotoOpenedFile` | Select Opened File | `FileChooserToolbar` |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `FileChooser.NewFile` | New File… | `FileChooserToolbar` |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `FileChooser.NewFolder` | New Directory… | `FileChooserToolbar` | `alt INSERT`, `control N` | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `FileChooser.Refresh` | Refresh | `FileChooserToolbar` | as `Synchronize` | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `FileChooser.ShowHidden` | Show Hidden Files and Directories | `FileChooserSettings` |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `FileChooser.ToggleDetection` | Detect Project Directories | `FileChooserSettings` |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `FileChooser.TogglePathBar` | Show/Hide Path Input Field | `FileChooserSettings` | `control P` | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `ForceRefresh` | Force Refresh |  | `control shift F5` | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-api` |
| `InvalidateCaches` | Invalidate Caches… | `FileMenu` |  | platform/01-vfs-and-documents.md PLAT-VFS | `platform/platform-impl` |
| `LightEditOpenFileInProject` | (set in code) | `LightEditModePopup` |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `NewScratchBuffer` | New Scratch Buffer |  |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/scratch` |
| `NewScratchFile` | Scratch File | `NewGroup`, `ConsoleEditorPopupMenu` | `control alt shift INSERT` | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/scratch` |
| `OpenInAssociatedApplication` | Open in Associated Application | `RevealGroup` |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `OpenInBrowser` | Open in Default Browser |  |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `Print` | Print… | `PrintExportGroup` |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/lang-impl` |
| `Refresh` | Refresh |  | `control F5` | platform/01-vfs-and-documents.md PLAT-VFS-038 | `platform/platform-impl` |
| `RemoveBom` | Remove BOM | `RemoveBom.Group`, `EncodingPanelActions` |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `RevealIn` | Show in File Manager | `RevealGroup` |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `SaveAll` | Save All | `FileMenu`, `MainToolBar` | `control S` | platform/01-vfs-and-documents.md PLAT-VFS | `platform/platform-impl` |
| `SaveAsNewFormat` | Save as Directory-Based Format… | `ExportImportGroup` |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `java/java-backend` |
| `SaveDocument` | Save File |  |  | platform/01-vfs-and-documents.md PLAT-VFS | `platform/platform-impl` |
| `SaveFileAsTemplate` | Save File as Template… | `FileMenu` |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/lang-impl` |
| `Scratch.ChangeLanguage` | Change File Language | `EditorPopupMenu2`, `ProjectViewPopupMenuModifyGroup`, `EditorTabPopupMenuEx` |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/scratch` |
| `Scratch.ExportToScratch` | Export to Scratch File |  |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/scratch` |
| `Scratch.ShowFilesPopup` | Show Scratch Files |  |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/scratch` |
| `Synchronize` | Reload All from Disk | `FileMenu`, `MainToolBar` | `control alt Y` | platform/01-vfs-and-documents.md PLAT-VFS | `platform/platform-impl` |
| `SynchronizeCurrentFile` | Reload from Disk | `ProjectViewPopupMenu`, `NavbarPopupMenu` |  | platform/01-vfs-and-documents.md PLAT-VFS-102 | `platform/lang-impl` |
| `ToggleReadOnlyAttribute` | Toggle Read-Only Attribute | `FilePropertiesGroup` |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/platform-impl` |
| `WelcomeFileChangeLanguage` | Change File Language | `EditorPopupMenu2` | as `Scratch.ChangeLanguage` | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/scratch` |
| `WslSyncAction` | WSL Sync |  |  | framework: platform/01-vfs-and-documents.md (PLAT-VFS) | `platform/wsl-impl` |

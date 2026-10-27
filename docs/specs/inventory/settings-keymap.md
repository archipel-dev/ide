# Settings, keymap, appearance and schemes

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (24)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `ChangeColorScheme` | Editor Color Scheme | `ChangeScheme` |  | MISSING | `platform/platform-impl` |
| `ChangeKeymap` | Keymap | `ChangeScheme`, `SettingsEntryPointGroup` |  | MISSING | `platform/platform-impl` |
| `ChangeLaf` | Theme | `ChangeScheme`, `SettingsEntryPointGroup` |  | MISSING | `platform/platform-impl` |
| `ConfigureIcs` | Settings Repository… | `ExportImportGroup`, `WelcomeScreen.Configure` |  | MISSING | `plugins/settings-repository` |
| `CustomizeMainToolbarAction` | Customize Toolbar… | `CustomizeMainToolbarGroup` |  | MISSING | `platform/platform-impl` |
| `CustomizeUI` | (set in code) | `MainToolbarPopupActions` |  | MISSING | `platform/platform-impl` |
| `ExportSettings` | Export Settings… | `ExportImportGroup`, `WelcomeScreen.Configure` |  | MISSING | `platform/configuration-store-impl` |
| `ImportSettings` | Import Settings… | `ExportImportGroup`, `WelcomeScreen.Configure` |  | MISSING | `platform/configuration-store-impl` |
| `MergeSettings` | Merge | `SyncSettings` |  | MISSING | `plugins/settings-repository` |
| `QuickChangeScheme` | Quick Switch Scheme… | `ViewMenu` | `control BACK_QUOTE` | MISSING | `platform/platform-impl` |
| `ResetToMySettings` | Overwrite Remote | `SyncSettings` |  | MISSING | `plugins/settings-repository` |
| `ResetToTheirsSettings` | Overwrite Local | `SyncSettings` |  | MISSING | `plugins/settings-repository` |
| `RestoreDefaultSettings` | Restore Default Settings… | `ExportImportGroup`, `WelcomeScreen.Configure` |  | MISSING | `platform/configuration-store-impl` |
| `Settings.Paste` | Paste | `EditorActions` | as `$Paste` | MISSING | `platform/platform-impl` |
| `SettingsSyncOpenSettingsAction` | Backup and Sync… | `ExportImportGroup` |  | MISSING | `platform/settings-sync-core` |
| `SettingsSyncSeparator` | (set in code) | `SettingsEntryPointGroup` |  | MISSING | `platform/settings-sync-core` |
| `SettingsSyncStatusAction` | Backup and Sync | `SettingsEntryPointGroup` |  | MISSING | `platform/settings-sync-core` |
| `ShowSettings` | (set in code) | `FileMainSettingsGroup`, `WelcomeScreen.Configure`, `WelcomeScreen.Options` (+1) | `control alt S` | MISSING | `platform/platform-impl` |
| `ShowSettingsChangesLogFile` | (set in code) |  |  | MISSING | `platform/platform-impl` |
| `ShowSettingsWithAddedPattern` | Disable Hints for Method | `ParameterNameHints` |  | MISSING | `platform/lang-impl` |
| `internal.settings.v2.DumpSettingsContext` | Dump Settings Context |  |  | MISSING | `platform/settings.v2` |
| `settingsSync.history` | Backup and Sync History |  |  | MISSING | `platform/settings-sync-core` |
| `settingsSync.manualPush` | Push Settings | `SettingsSync` |  | MISSING | `platform/settings-sync-core` |
| `settingsSync.troubleShoot` | Backup and Sync Troubleshooting |  |  | MISSING | `plugins/settings-sync` |

## Settings pages (39)

| Id | Display name | Group | Parent | Scope | Spec |
|---|---|---|---|---|---|
| `application.passwordSafe` | Passwords |  | preferences.general | application | MISSING |
| `preferences.editor` | General | editor |  | application | MISSING |
| `preferences.editor.code.editing` | Code Editing | editor |  | application | MISSING |
| `Console` | Console |  | preferences.editor | application | MISSING |
| `preferences.fileTypes` | File Types | editor |  | application | MISSING |
| `http.proxy` | HTTP Proxy |  | preferences.general | application | MISSING |
| `com.intellij.platform.ide.impl.presentationAssistant.PresentationAssistantConfigurable` | Presentation Assistant | appearance |  | application | MISSING |
| `trusted.hosts` | Trusted Locations | appearance |  | application | MISSING |
| `preferences.lookFeel` | Appearance | appearance |  | application | codeinsight/03-inspections.md CI-INS-064 |
| `preferences.keymap` | Keymap | root |  | application | MISSING |
| `reference.idesettings.quicklists` | Quick Lists | appearance |  | application | MISSING |
| `preferences.customizations` | Menus and Toolbars | appearance |  | application | MISSING |
| `reference.settings.ide.settings.notifications` | Notifications | appearance |  | application | MISSING |
| `preferences.accessibility` | Accessibility | appearance |  | application | java/02-resolve.md JAVA-RES-042 |
| `preferences.pluginManager` | Plugins | root |  | application | MISSING |
| `consents` | Data Sharing |  | preferences.general | application | MISSING |
| `ide.date.format` | Date Formats |  | preferences.general | application | MISSING |
| `preferences.language.and.region` | Language and Region |  | preferences.general | application | MISSING |
| `http.certificates` | Server Certificates |  | preferences.general | application | MISSING |
| `editor.preferences.fonts.default` | Font | editor |  | application | MISSING |
| `app.console.font` | Console Font | editor | editor.preferences.fonts.default | application | MISSING |
| `editor.breadcrumbs` | Breadcrumbs |  | preferences.editor | application | platform/08-language-extension-points.md PLAT-LANG-088 |
| `editor.stickyLines` | Sticky Lines |  | preferences.editor | application | MISSING |
| `reference.settings.ide.settings.web.browsers` | Web Browsers and Preview | tools |  | application | MISSING |
| `advanced.settings` | Advanced Settings | root |  | application | MISSING |
| `reference.settingsdialog.IDE.editor.colors` | Color Scheme | editor |  | application | MISSING |
| `preferences.updates` | Updates |  | preferences.general | application | MISSING |
| `settings.sync` | Backup and Sync | root |  | application | MISSING |
| `com.intellij.minimap` | Minimap |  | editor | application | MISSING |
| `ics` | Settings Repository | tools |  | application | MISSING |
| `editor.reader.mode` | Reader Mode | editor |  | project | MISSING |
| `File.Encoding` | File Encodings | editor |  | project | MISSING |
| `reference.settings.ide.settings.file-colors` | File Colors | appearance |  | project | MISSING |
| `preferences.pathVariables` | Path Variables | appearance |  | project | MISSING |
| `preferences.general` | System Settings | appearance |  | project | MISSING |
| `preferences.externalDependencies` | Required Plugins | appearance |  | project | MISSING |
| `actions.on.save` | Actions on Save | tools |  | project | codeinsight/08-optimize-imports-and-reformat.md CI-OPT-043 |
| `lsp.servers` | Language Servers |  | language | project | MISSING |
| `tasks` | Tasks | tools |  | project | MISSING |

## Tool windows (4)

| Id | Title | Anchor | Flags | Factory | Spec |
|---|---|---|---|---|---|
| `Settings` | Settings | left |  | SettingsToolWindowFactory | MISSING |
| `UI Theme Color Picker` | UI Theme Color Picker | right |  | UiThemeColorPickerToolWindowFactory | MISSING |
| `Backup and Sync History` | Backup and Sync History | left |  | SettingsHistoryToolWindowFactory | MISSING |
| `Commander` | Commander | right |  | CommanderToolWindowFactory | MISSING |

## Status bar widgets (14)

| Id | Factory | Order | Spec |
|---|---|---|---|
| `largeFileEncodingWidget` | `LargeFileEncodingWidgetFactory` | after PowerSaveMode | MISSING |
| `CodeStyleStatusBarWidget` | `CodeStyleStatusBarWidgetFactory` | after InsertOverwrite | MISSING |
| `light.edit.large.file.encoding.widget` | `LightEditLargeFileEncodingWidgetFactory` | after LineSeparator | MISSING |
| `VfsRefresh` | `VfsRefreshIndicatorWidgetFactory` | first | MISSING |
| `Position` | `PositionPanelWidgetFactory` |  | MISSING |
| `LineSeparator` | `LineSeparatorWidgetFactory` | after Position | MISSING |
| `Encoding` | `EncodingPanelWidgetFactory` | after LineSeparator | MISSING |
| `PowerSaveMode` | `PowerSaveStatusWidgetFactory` | after Encoding | MISSING |
| `InsertOverwrite` | `ColumnSelectionModeWidgetFactory` | after PowerSaveMode | MISSING |
| `ReadOnlyAttribute` | `ReadOnlyAttributeWidgetFactory` | after InsertOverwrite | MISSING |
| `Notifications` | `NotificationWidgetFactory` | after ReadOnlyAttribute | MISSING |
| `FatalError` | `FatalErrorWidgetFactory` | after Notifications | MISSING |
| `Memory` | `MemoryIndicatorWidgetFactory` | last | MISSING |
| `settingsEntryPointWidget` | `StatusBarManager` | last | MISSING |

## Keymap registrations (26)

| Keymap | Plugin | Spec |
|---|---|---|
| `$default.xml` | platform/platform-impl | MISSING |
| `Mac OS X 10.5+.xml` | platform/platform-impl | MISSING |
| `Mac OS X.xml` | platform/platform-impl | MISSING |
| `macOS System Shortcuts.xml` | platform/platform-impl | MISSING |
| `Default for XWin.xml` | platform/platform-impl | MISSING |
| `Default for GNOME.xml` | platform/platform-impl | MISSING |
| `Default for KDE.xml` | platform/platform-impl | MISSING |
| `Emacs.xml` | platform/platform-impl | MISSING |
| `Sublime Text.xml` | platform/platform-impl | MISSING |
| `Sublime Text (Mac OS X).xml` | platform/platform-impl | MISSING |
| `Eclipse.xml` | plugins/keymaps | MISSING |
| `Eclipse (Mac OS X).xml` | plugins/keymaps | MISSING |
| `NetBeans 6.5.xml` | plugins/keymaps | MISSING |
| `QtCreator.xml` | plugins/keymaps | MISSING |
| `QtCreator (Mac OS X).xml` | plugins/keymaps | MISSING |
| `ReSharper.xml` | plugins/keymaps | MISSING |
| `ReSharper OSX.xml` | plugins/keymaps | MISSING |
| `Visual Assist.xml` | plugins/keymaps | MISSING |
| `Visual Assist OSX.xml` | plugins/keymaps | MISSING |
| `Visual Studio for Mac.xml` | plugins/keymaps | MISSING |
| `Visual Studio.xml` | plugins/keymaps | MISSING |
| `Visual Studio OSX.xml` | plugins/keymaps | MISSING |
| `Visual Studio 2022.xml` | plugins/keymaps | MISSING |
| `VSCode.xml` | plugins/keymaps | MISSING |
| `VSCode OSX.xml` | plugins/keymaps | MISSING |
| `Xcode.xml` | plugins/keymaps | MISSING |

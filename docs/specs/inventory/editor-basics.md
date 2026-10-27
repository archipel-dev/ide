# Editor basics (caret, selection, scrolling, typing, clipboard, undo)

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (149)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `$Copy` | Copy | `Console.AggregateView.PopupGroup`, `CutCopyPasteGroup`, `EditorPopupMenu` (+3) | `control C`, `control INSERT` | MISSING | `platform/platform-impl` |
| `$Cut` | Cut | `CutCopyPasteGroup`, `EditorPopupMenu`, `BasicEditorPopupMenu` (+2) | `control X`, `shift DELETE` | MISSING | `platform/platform-impl` |
| `$Delete` | Delete | `CommanderPopupMenu`, `ServiceViewItemPopup`, `HierarchyPopupMenu.Common` (+9) | `DELETE` | MISSING | `platform/platform-impl` |
| `$Paste` | Paste | `Console.TableResult.PopupGroup`, `PasteGroup`, `EditorPopupMenu` (+3) | `control V`, `shift INSERT` | MISSING | `platform/platform-impl` |
| `$Redo` | Redo | `EditMenu` | `control shift Z`, `alt shift BACK_SPACE` | MISSING | `platform/platform-impl` |
| `$SelectAll` | Select All | `EditSelectGroup` | `control A` | MISSING | `platform/platform-impl` |
| `$Undo` | Undo | `EditMenu` | `control Z`, `alt BACK_SPACE` | MISSING | `platform/platform-impl` |
| `ClearAllNotifications` | Clear All Notifications |  |  | MISSING | `platform/platform-impl` |
| `ClearIDEStatistics` | Clear IDE Statistics | `Internal.Ex` |  | MISSING | `platform/platform-impl` |
| `CopyAsPlainText` | Copy as Plain Text | `CutCopyPasteGroup`, `Copy.Paste.Special` |  | MISSING | `platform/lang-impl` |
| `CopyAsRichText` | Copy as Rich Text | `CutCopyPasteGroup`, `EditorPopupMenu` |  | MISSING | `platform/lang-impl` |
| `CopyContentRootPath` | Path from Content Root | `CopyFileReference` |  | MISSING | `platform/lang-impl` |
| `CopyFileName` | File Name | `CopyFileReference` |  | MISSING | `platform/lang-impl` |
| `CopyPathWithLineNumber` | Path with Line Number | `CopyFileReference` |  | MISSING | `platform/lang-impl` |
| `CopySettingsPath` | Copy Option Path |  |  | MISSING | `platform/platform-impl` |
| `CopySourceRootPath` | Path from Source Root | `CopyFileReference` |  | MISSING | `platform/lang-impl` |
| `CopyTBXReference` | Toolbox URL | `CopyExternalReferenceGroup` |  | MISSING | `platform/lang-impl` |
| `CopyUiLabel` | Copy UI Label | `UiInspectorGroup` | `mouse control alt button3`, `mouse control alt shift button3` | MISSING | `platform/platform-impl` |
| `CopyUrl` | Copy URL | `ConsoleEditorPopupMenu` |  | MISSING | `platform/platform-impl` |
| `EditorAddOrRemoveCaret` | Add or Remove Caret | `EditorActions` | `mouse alt shift button1` | MISSING | `platform/platform-api` |
| `EditorBackSpace` | Backspace | `EditorActions` | `BACK_SPACE`, `shift BACK_SPACE` | MISSING | `platform/platform-impl` |
| `EditorBackwardParagraph` | Move Caret Backward a Paragraph | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorBackwardParagraphWithSelection` | Move Caret Backward a Paragraph with Selection | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorBreadcrumbsHideBoth` | Don't Show | `EditorBreadcrumbsSettings` |  | MISSING | `platform/platform-impl` |
| `EditorBreadcrumbsShowAbove` | Top | `EditorBreadcrumbsSettings` |  | MISSING | `platform/platform-impl` |
| `EditorBreadcrumbsShowBelow` | Bottom | `EditorBreadcrumbsSettings` |  | MISSING | `platform/platform-impl` |
| `EditorCenterView` | Toggle Center View | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorCodeBlockEnd` | Move Caret to Code Block End | `CodeInsightEditorActions` | `control CLOSE_BRACKET` | MISSING | `platform/lang-impl` |
| `EditorCodeBlockEndWithSelection` | Move Caret to Code Block End with Selection | `CodeInsightEditorActions` | `control shift CLOSE_BRACKET` | MISSING | `platform/lang-impl` |
| `EditorCodeBlockStart` | Move Caret to Code Block Start | `CodeInsightEditorActions` | `control OPEN_BRACKET` | MISSING | `platform/lang-impl` |
| `EditorCodeBlockStartWithSelection` | Move Caret to Code Block Start with Selection | `CodeInsightEditorActions` | `control shift OPEN_BRACKET` | MISSING | `platform/lang-impl` |
| `EditorCompleteStatement` | Complete Current Statement | `CodeCompletionGroup`, `CodeInsightEditorActions` | `control shift ENTER` | codeinsight/06-completion.md | `platform/lang-impl` |
| `EditorContextInfo` | Context Info | `CodeEditorViewGroup` | `alt Q` | MISSING | `platform/lang-impl` |
| `EditorCopy` | Copy | `EditorActions` | as `$Copy` | MISSING | `platform/platform-impl` |
| `EditorCut` | Cut | `EditorActions`, `Git.Branches.Popup.SpeedSearch`, `Terminal.PromptContextMenu` | as `$Cut` | MISSING | `platform/platform-impl` |
| `EditorCutLineBackward` | Cut Line Backward | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorCutLineEnd` | Cut up to Line End | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorDecreaseFontSize` | Decrease Font Size | `EditorActions`, `EditorToggleActions` |  | MISSING | `platform/platform-impl` |
| `EditorDecreaseFontSizeGlobal` | Decrease Font Size in All Editors | `EditorActions`, `ViewMenu` | `alt shift COMMA` | MISSING | `platform/platform-impl` |
| `EditorDelete` | Delete | `EditorActions` | as `$Delete` | MISSING | `platform/platform-impl` |
| `EditorDeleteLine` | Delete Line | `EditorActions` | `control Y` | MISSING | `platform/platform-impl` |
| `EditorDeleteToLineEnd` | Delete to Line End | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorDeleteToLineStart` | Delete to Line Start | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorDeleteToWordEnd` | Delete to Word End | `EditorActions` | `control DELETE` | MISSING | `platform/platform-impl` |
| `EditorDeleteToWordEndInDifferentHumpsMode` | Delete to Word End in Different "CamelHumps" Mode | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorDeleteToWordStart` | Delete to Word Start | `EditorActions` | `control BACK_SPACE` | MISSING | `platform/platform-impl` |
| `EditorDeleteToWordStartInDifferentHumpsMode` | Delete to Word Start in Different "CamelHumps" Mode | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorDown` | Down | `EditorActions` | `DOWN` | MISSING | `platform/platform-impl` |
| `EditorDownWithSelection` | Down with Selection | `EditorActions` | `shift DOWN` | MISSING | `platform/platform-impl` |
| `EditorDuplicate` | Duplicate Line or Selection | `EditorActions`, `EditSmartGroup` | `control D` | MISSING | `platform/platform-impl` |
| `EditorDuplicateLines` | Duplicate Entire Lines | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorEnter` | Enter | `EditorActions` | `ENTER` | MISSING | `platform/platform-impl` |
| `EditorFocusGutter` | Focus Gutter (Accessibility) | `EditorActions` | `alt shift 6, F` | MISSING | `platform/platform-impl` |
| `EditorForwardParagraph` | Move Caret Forward a Paragraph | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorForwardParagraphWithSelection` | Move Caret Forward a Paragraph with Selection | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorGutterToggleGlobalIndentLines` | Show Indent Guides | `EditorGutterPopupMenu.Appearance` |  | MISSING | `platform/platform-impl` |
| `EditorGutterToggleGlobalLineNumbers` | Show Line Numbers | `EditorGutterPopupMenu.Appearance` |  | MISSING | `platform/platform-impl` |
| `EditorGutterToggleGlobalStickyLines` | Show Sticky Lines | `EditorGutterPopupMenu.Appearance` |  | MISSING | `platform/platform-impl` |
| `EditorHungryBackSpace` | Hungry Backspace | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorIncreaseFontSize` | Increase Font Size | `EditorActions`, `EditorToggleActions` |  | MISSING | `platform/platform-impl` |
| `EditorIncreaseFontSizeGlobal` | Increase Font Size in All Editors | `EditorActions`, `ViewMenu` | `alt shift PERIOD` | MISSING | `platform/platform-impl` |
| `EditorIndentLineOrSelection` | Indent Line or Selection | `EditorActions` |  | MISSING | `platform/lang-impl` |
| `EditorJoinLines` | Join Lines | `EditorActions`, `EditSmartGroup` | `control shift J` | MISSING | `platform/platform-impl` |
| `EditorKillRegion` | Kill Selected Region | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorKillRingSave` | Save to Kill Ring | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorKillToWordEnd` | Kill to Word End | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorKillToWordStart` | Kill to Word Start | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorLeft` | Left | `EditorActions` | `LEFT` | MISSING | `platform/platform-impl` |
| `EditorLeftWithSelection` | Left with Selection | `EditorActions` | `shift LEFT` | MISSING | `platform/platform-impl` |
| `EditorLineEnd` | Move Caret to Line End | `EditorActions` | `END` | MISSING | `platform/platform-impl` |
| `EditorLineEndWithSelection` | Move Caret to Line End with Selection | `EditorActions` | `shift END` | MISSING | `platform/platform-impl` |
| `EditorLineStart` | Move Caret to Line Start | `EditorActions` | `HOME` | MISSING | `platform/platform-impl` |
| `EditorLineStartWithSelection` | Move Caret to Line Start with Selection | `EditorActions` | `shift HOME` | MISSING | `platform/platform-impl` |
| `EditorMatchBrace` | Move Caret to Matching Brace | `CodeInsightEditorActions`, `NavigateInFileGroup` | `control shift M` | MISSING | `platform/lang-impl` |
| `EditorMoveDownAndScroll` | Move Down and Scroll | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorMoveDownAndScrollWithSelection` | Move Down and Scroll with Selection | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorMoveToPageBottom` | Move Caret to Page Bottom | `EditorActions` | `control PAGE_DOWN` | MISSING | `platform/platform-impl` |
| `EditorMoveToPageBottomWithSelection` | Move Caret to Page Bottom with Selection | `EditorActions` | `control shift PAGE_DOWN` | MISSING | `platform/platform-impl` |
| `EditorMoveToPageTop` | Move Caret to Page Top | `EditorActions` | `control PAGE_UP` | MISSING | `platform/platform-impl` |
| `EditorMoveToPageTopWithSelection` | Move Caret to Page Top with Selection | `EditorActions` | `control shift PAGE_UP` | MISSING | `platform/platform-impl` |
| `EditorMoveUpAndScroll` | Move Up and Scroll | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorMoveUpAndScrollWithSelection` | Move Up and Scroll with Selection | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorNextWord` | Move Caret to Next Word | `EditorActions` | `control RIGHT` | MISSING | `platform/platform-impl` |
| `EditorNextWordInDifferentHumpsMode` | Move Caret to Next Word in Different "CamelHumps" Mode | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorNextWordInDifferentHumpsModeWithSelection` | Move Caret to Next Word with Selection in Different "CamelHumps" Mode | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorNextWordWithSelection` | Move Caret to Next Word with Selection | `EditorActions` | `control shift RIGHT` | MISSING | `platform/platform-impl` |
| `EditorPageDown` | Page Down | `EditorActions` | `PAGE_DOWN` | MISSING | `platform/platform-impl` |
| `EditorPageDownWithSelection` | Page Down with Selection | `EditorActions` | `shift PAGE_DOWN` | MISSING | `platform/platform-impl` |
| `EditorPageUp` | Page Up | `EditorActions` | `PAGE_UP` | MISSING | `platform/platform-impl` |
| `EditorPageUpWithSelection` | Page Up with Selection | `EditorActions` | `shift PAGE_UP` | MISSING | `platform/platform-impl` |
| `EditorPaste` | Paste | `EditorActions`, `Git.Branches.Popup.SpeedSearch` | as `$Paste` | MISSING | `platform/platform-impl` |
| `EditorPasteFromX11` | Paste from X Clipboard | `EditorActions` | `mouse button2` | MISSING | `platform/platform-impl` |
| `EditorPasteSimple` | Paste as Plain Text | `EditorActions`, `PasteGroup`, `Copy.Paste.Special` | `control alt shift V` | MISSING | `platform/platform-impl` |
| `EditorPreviousWord` | Move Caret to Previous Word | `EditorActions` | `control LEFT` | MISSING | `platform/platform-impl` |
| `EditorPreviousWordInDifferentHumpsMode` | Move Caret to Previous Word in Different "CamelHumps" Mode | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorPreviousWordInDifferentHumpsModeWithSelection` | Move Caret to Previous Word with Selection in Different "CamelHumps" Mode | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorPreviousWordWithSelection` | Move Caret to Previous Word with Selection | `EditorActions` | `control shift LEFT` | MISSING | `platform/platform-impl` |
| `EditorResetFontSize` | Reset Font Size | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorResetFontSizeGlobal` | Reset Font Size in All Editors | `ViewMenu` |  | MISSING | `platform/platform-impl` |
| `EditorReverseLines` | Reverse Lines | `EditorActions`, `EditSmartGroup` |  | MISSING | `platform/platform-impl` |
| `EditorRight` | Right | `EditorActions` | `RIGHT` | MISSING | `platform/platform-impl` |
| `EditorRightWithSelection` | Right with Selection | `EditorActions` | `shift RIGHT` | MISSING | `platform/platform-impl` |
| `EditorScrollBottom` | Scroll to Bottom | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorScrollDown` | Scroll Down | `EditorActions` | `control DOWN` | MISSING | `platform/platform-impl` |
| `EditorScrollDownAndMove` | Scroll Down and Move If Necessary | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorScrollLeft` | Scroll Left | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorScrollRight` | Scroll Right | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorScrollToCenter` | Scroll to Center | `EditorActions` | `control M` | MISSING | `platform/platform-impl` |
| `EditorScrollTop` | Scroll to Top | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorScrollUp` | Scroll Up | `EditorActions` | `control UP` | MISSING | `platform/platform-impl` |
| `EditorScrollUpAndMove` | Scroll Up and Move If Necessary | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorSelectLine` | Extend Line Selection | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorSelectSingleLineAtCaret` | Select Single Line at Caret | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorSelectWord` | Extend Selection | `LangCodeInsightActions`, `EditSelectWordGroup` | `control W` | MISSING | `platform/lang-impl` |
| `EditorSelectWordAtCurrentCaret` | Extend Selection | `LangCodeInsightActions` |  | MISSING | `platform/lang-impl` |
| `EditorSetContentBasedBidiTextDirection` | Content-Based | `EditorBidiTextDirection` |  | MISSING | `platform/platform-impl` |
| `EditorSetLtrBidiTextDirection` | Left-to-Right | `EditorBidiTextDirection` |  | MISSING | `platform/platform-impl` |
| `EditorSetRtlBidiTextDirection` | Right-to-Left | `EditorBidiTextDirection` |  | MISSING | `platform/platform-impl` |
| `EditorShowGutterIconTooltip` | Show Gutter Icon Tooltip (Accessibility) | `EditorActions` | `alt shift 6, T` | MISSING | `platform/platform-impl` |
| `EditorSortLines` | Sort Lines | `EditorActions`, `EditSmartGroup` |  | MISSING | `platform/platform-impl` |
| `EditorSplitLine` | Split Line | `EditorActions` | `control ENTER` | MISSING | `platform/platform-impl` |
| `EditorStartNewLine` | Start New Line | `EditorActions` | `shift ENTER` | MISSING | `platform/platform-impl` |
| `EditorStartNewLineBefore` | Start New Line Before Current | `EditorActions` | `control alt ENTER` | MISSING | `platform/platform-impl` |
| `EditorStickyConfigure` | Configure Sticky Lines… | `EditorStickyLinesSettings` |  | MISSING | `platform/platform-impl` |
| `EditorStickyLinesDisable` | Disable | `EditorStickyLinesSettings` |  | MISSING | `platform/platform-impl` |
| `EditorStickyLinesDisableForLang` | Disable for Language | `EditorStickyLinesSettings` |  | MISSING | `platform/platform-impl` |
| `EditorSwapSelectionBoundaries` | Swap Selection Boundaries | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorTextEnd` | Move Caret to Text End | `EditorActions` | `control END` | MISSING | `platform/platform-impl` |
| `EditorTextEndWithSelection` | Move Caret to Text End with Selection | `EditorActions` | `control shift END` | MISSING | `platform/platform-impl` |
| `EditorTextStart` | Move Caret to Text Start | `EditorActions` | `control HOME` | MISSING | `platform/platform-impl` |
| `EditorTextStartWithSelection` | Move Caret to Text Start with Selection | `EditorActions` | `control shift HOME` | MISSING | `platform/platform-impl` |
| `EditorToggleCase` | Toggle Case | `EditorActions`, `EditSmartGroup` | `control shift U` | MISSING | `platform/platform-impl` |
| `EditorToggleInsertState` | Toggle Insert/Overwrite | `EditorActions` | `INSERT` | MISSING | `platform/platform-impl` |
| `EditorToggleShowBreadcrumbs` | Show Breadcrumbs | `EditorToggleActions` |  | MISSING | `platform/platform-impl` |
| `EditorToggleShowGutterIcons` | Show Gutter Icons | `EditorToggleActions` |  | MISSING | `platform/platform-impl` |
| `EditorToggleShowIndentLines` | Show Indent Guides | `EditorToggleActions` |  | MISSING | `platform/platform-impl` |
| `EditorToggleShowLineNumbers` | Show Line Numbers | `EditorToggleActions` |  | MISSING | `platform/platform-impl` |
| `EditorToggleShowWhitespaces` | Show Whitespaces | `EditorToggleActions` |  | MISSING | `platform/platform-impl` |
| `EditorToggleStickySelection` | Toggle Sticky Selection | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorToggleUseSoftWraps` | Soft-Wrap | `EditorToggleActions`, `EditorGutterPopupMenu`, `XDebugger.Evaluate.Code.Fragment.Editor.Popup` |  | MISSING | `platform/platform-impl` |
| `EditorToggleUseSoftWrapsInPreview` | Soft-Wrap Preview Editor | `EditorContextBarMenu` |  | MISSING | `platform/platform-impl` |
| `EditorTranspose` | Transpose | `EditorActions`, `EditSmartGroup` |  | MISSING | `platform/platform-impl` |
| `EditorUniqueLines` | Delete Duplicate Lines | `EditorActions` |  | MISSING | `platform/platform-impl` |
| `EditorUp` | Up | `EditorActions` | `UP` | MISSING | `platform/platform-impl` |
| `EditorUpWithSelection` | Up with Selection | `EditorActions` | `shift UP` | MISSING | `platform/platform-impl` |
| `PasteMultiple` | Paste from History… | `PasteGroup`, `Copy.Paste.Special` | `control shift V`, `control shift INSERT` | MISSING | `platform/platform-impl` |
| `ShowIntentionActions` | Show Context Actions | `ShowIntentionsGroup` | `alt ENTER` | codeinsight/04-intentions-and-alt-enter.md CI-IMP-040 | `platform/lang-impl` |
| `ToggleRenderedDocPresentation` | Toggle Rendered View |  | `control alt Q` | MISSING | `platform/lang-impl` |
| `ToggleRenderedDocPresentationForAll` | Render All Doc Comments | `DocCommentGutterIconContextMenu` |  | MISSING | `platform/lang-impl` |

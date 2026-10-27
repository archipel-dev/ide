# Generic UI component actions (list, tree, table, popup, scroll-pane navigation)

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (91)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `ClickLink` | Click Link |  | as `GotoDeclaration` | MISSING | `platform/lang-impl` |
| `CloseGotItTooltip` | Close Got It Tooltip |  | `ESCAPE` | MISSING | `platform/platform-api` |
| `CollapsiblePanel-toggle` | Expand/Collapse Collapsible Panel |  | `SPACE` | MISSING | `platform/platform-api` |
| `EmojiAndSymbols` | Emoji \\&\\& Symbols | `EditMenu` |  | MISSING | `platform/platform-impl` |
| `FullyExpandTreeNode` | Fully Expand Tree Node |  | `MULTIPLY` | MISSING | `platform/platform-impl` |
| `List-scrollDown` | Scroll Page Down and Select Row | `ListActions` | as `EditorPageDown` | MISSING | `platform/platform-impl` |
| `List-scrollDownExtendSelection` | Scroll Page Down and Extend Selection | `ListActions` | as `EditorPageDownWithSelection` | MISSING | `platform/platform-impl` |
| `List-scrollUp` | Scroll Page Up and Select Row | `ListActions` | as `EditorPageUp` | MISSING | `platform/platform-impl` |
| `List-scrollUpExtendSelection` | Scroll Page Up and Extend Selection | `ListActions` | as `EditorPageUpWithSelection` | MISSING | `platform/platform-impl` |
| `List-selectFirstRow` | Select First Row | `ListActions` | as `EditorTextStart` | MISSING | `platform/platform-impl` |
| `List-selectFirstRowExtendSelection` | Extend Selection to First Row | `ListActions` | as `EditorTextStartWithSelection` | MISSING | `platform/platform-impl` |
| `List-selectLastRow` | Select Last Row | `ListActions` | as `EditorTextEnd` | MISSING | `platform/platform-impl` |
| `List-selectLastRowExtendSelection` | Extend Selection to Last Row | `ListActions` | as `EditorTextEndWithSelection` | MISSING | `platform/platform-impl` |
| `List-selectNextColumn` | Select Next Column | `ListActions` | as `EditorRight` | MISSING | `platform/platform-impl` |
| `List-selectNextColumnExtendSelection` | Extend Selection to Next Column | `ListActions` | as `EditorRightWithSelection` | MISSING | `platform/platform-impl` |
| `List-selectNextRow` | Select Next Row | `ListActions` | as `EditorDown` | MISSING | `platform/platform-impl` |
| `List-selectNextRowExtendSelection` | Extend Selection with Next Row | `ListActions` | as `EditorDownWithSelection` | MISSING | `platform/platform-impl` |
| `List-selectPreviousColumn` | Select Previous Column | `ListActions` | as `EditorLeft` | MISSING | `platform/platform-impl` |
| `List-selectPreviousColumnExtendSelection` | Extend Selection to Previous Column | `ListActions` | as `EditorLeftWithSelection` | MISSING | `platform/platform-impl` |
| `List-selectPreviousRow` | Select Previous Row | `ListActions` | as `EditorUp` | MISSING | `platform/platform-impl` |
| `List-selectPreviousRowExtendSelection` | Extend Selection with Previous Row | `ListActions` | as `EditorUpWithSelection` | MISSING | `platform/platform-impl` |
| `OpenElementInNewWindow` | Open in New Editor Window |  |  | MISSING | `platform/platform-impl` |
| `PopupMenu-cancel` | Close Context Menu | `PopupMenuActions` | as `EditorEscape` | MISSING | `platform/platform-api` |
| `PopupMenu-return` | Invoke Selected Action | `PopupMenuActions` | as `EditorEnter` | MISSING | `platform/platform-api` |
| `PopupMenu-selectChild` | Select Child Menu Item | `PopupMenuActions` | as `EditorRight` | MISSING | `platform/platform-api` |
| `PopupMenu-selectNext` | Select Next Menu Item | `PopupMenuActions` | as `EditorDown` | MISSING | `platform/platform-api` |
| `PopupMenu-selectParent` | Select Parent Menu Item | `PopupMenuActions` | as `EditorLeft` | MISSING | `platform/platform-api` |
| `PopupMenu-selectPrevious` | Select Previous Menu Item | `PopupMenuActions` | as `EditorUp` | MISSING | `platform/platform-api` |
| `ScrollPane-scrollDown` | Scroll Page Down | `ScrollPaneActions` |  | MISSING | `platform/platform-impl` |
| `ScrollPane-scrollEnd` | Scroll End | `ScrollPaneActions` |  | MISSING | `platform/platform-impl` |
| `ScrollPane-scrollHome` | Scroll Home | `ScrollPaneActions` |  | MISSING | `platform/platform-impl` |
| `ScrollPane-scrollLeft` | Scroll Page Left | `ScrollPaneActions` |  | MISSING | `platform/platform-impl` |
| `ScrollPane-scrollRight` | Scroll Page Right | `ScrollPaneActions` |  | MISSING | `platform/platform-impl` |
| `ScrollPane-scrollUp` | Scroll Page Up | `ScrollPaneActions` |  | MISSING | `platform/platform-impl` |
| `ScrollPane-unitScrollDown` | Scroll Down | `ScrollPaneActions` |  | MISSING | `platform/platform-impl` |
| `ScrollPane-unitScrollLeft` | Scroll Left | `ScrollPaneActions` |  | MISSING | `platform/platform-impl` |
| `ScrollPane-unitScrollRight` | Scroll Right | `ScrollPaneActions` |  | MISSING | `platform/platform-impl` |
| `ScrollPane-unitScrollUp` | Scroll Up | `ScrollPaneActions` |  | MISSING | `platform/platform-impl` |
| `ScrollTreeToCenter` | Scroll to Center |  | as `EditorScrollToCenter` | MISSING | `platform/platform-impl` |
| `SegmentedButton-left` | Select Previous Segmented Button |  | as `EditorLeft` | MISSING | `platform/platform-api` |
| `SegmentedButton-right` | Select Next Segmented Button |  | as `EditorRight` | MISSING | `platform/platform-api` |
| `SetShortcutAction` | Add Shortcut… |  | as `ShowIntentionActions` | MISSING | `platform/lang-impl` |
| `ShowColorPicker` | Show Color Picker | `Images.EditorToolbar` |  | MISSING | `platform/platform-impl` |
| `ShowPopupMenu` | Show Context Menu |  | `CONTEXT_MENU` | MISSING | `platform/platform-impl` |
| `SmartSelect` | Extend Selection |  | as `EditorSelectWord` | MISSING | `platform/platform-impl` |
| `SmartUnSelect` | Shrink Selection |  | as `EditorUnSelectWord` | MISSING | `platform/platform-impl` |
| `SpeedSearch` | Speed Search |  | as `Find` | MISSING | `platform/platform-impl` |
| `Table-scrollDownChangeSelection` | Scroll Page Down and Select Row | `TableActions` | as `EditorPageDown` | MISSING | `platform/platform-impl` |
| `Table-scrollDownExtendSelection` | Scroll Page Down and Extend Selection | `TableActions` | as `EditorPageDownWithSelection` | MISSING | `platform/platform-impl` |
| `Table-scrollUpChangeSelection` | Scroll Page Up and Select Row | `TableActions` | as `EditorPageUp` | MISSING | `platform/platform-impl` |
| `Table-scrollUpExtendSelection` | Scroll Page Up and Extend Selection | `TableActions` | as `EditorPageUpWithSelection` | MISSING | `platform/platform-impl` |
| `Table-selectFirstColumn` | Select First Column | `TableActions` | as `EditorPreviousWord` | MISSING | `platform/platform-impl` |
| `Table-selectFirstColumnExtendSelection` | Extend Selection to First Column | `TableActions` | as `EditorPreviousWordWithSelection` | MISSING | `platform/platform-impl` |
| `Table-selectFirstRow` | Select First Row | `TableActions` | `control UP` | MISSING | `platform/platform-impl` |
| `Table-selectFirstRowExtendSelection` | Extend Selection to First Row | `TableActions` | `control shift UP` | MISSING | `platform/platform-impl` |
| `Table-selectLastColumn` | Select Last Column | `TableActions` | as `EditorNextWord` | MISSING | `platform/platform-impl` |
| `Table-selectLastColumnExtendSelection` | Extend Selection to Last Column | `TableActions` | as `EditorNextWordWithSelection` | MISSING | `platform/platform-impl` |
| `Table-selectLastRow` | Select Last Row | `TableActions` | `control DOWN` | MISSING | `platform/platform-impl` |
| `Table-selectLastRowExtendSelection` | Extend Selection to Last Row | `TableActions` | `control shift DOWN` | MISSING | `platform/platform-impl` |
| `Table-selectNextColumn` | Select Next Column | `TableActions` | as `EditorRight` | MISSING | `platform/platform-impl` |
| `Table-selectNextColumnExtendSelection` | Extend Selection to Next Column | `TableActions` | as `EditorRightWithSelection` | MISSING | `platform/platform-impl` |
| `Table-selectNextRow` | Select Next Row | `TableActions` | as `EditorDown` | MISSING | `platform/platform-impl` |
| `Table-selectNextRowExtendSelection` | Extend Selection with Next Row | `TableActions` | as `EditorDownWithSelection` | MISSING | `platform/platform-impl` |
| `Table-selectPreviousColumn` | Select Previous Column | `TableActions` | as `EditorLeft` | MISSING | `platform/platform-impl` |
| `Table-selectPreviousColumnExtendSelection` | Extend Selection to Previous Column | `TableActions` | as `EditorLeftWithSelection` | MISSING | `platform/platform-impl` |
| `Table-selectPreviousRow` | Select Previous Row | `TableActions` | as `EditorUp` | MISSING | `platform/platform-impl` |
| `Table-selectPreviousRowExtendSelection` | Extend Selection with Previous Row | `TableActions` | as `EditorUpWithSelection` | MISSING | `platform/platform-impl` |
| `Table-startEditing` | Edit Table Cell | `TableActions` | `F2` | MISSING | `platform/platform-impl` |
| `TextComponent.ClearAction` | Clear Text |  |  | MISSING | `platform/platform-api` |
| `Tree-scrollDownChangeSelection` | Scroll Page Down and Select Node | `TreeActions` | as `EditorPageDown` | MISSING | `platform/platform-impl` |
| `Tree-scrollDownExtendSelection` | Scroll Page Down and Extend Selection | `TreeActions` | as `EditorPageDownWithSelection` | MISSING | `platform/platform-impl` |
| `Tree-scrollUpChangeSelection` | Scroll Page Up and Select Node | `TreeActions` | as `EditorPageUp` | MISSING | `platform/platform-impl` |
| `Tree-scrollUpExtendSelection` | Scroll Page Up and Extend Selection | `TreeActions` | as `EditorPageUpWithSelection` | MISSING | `platform/platform-impl` |
| `Tree-selectAll` | Select All Nodes | `TreeActions` | as `$SelectAll` | MISSING | `platform/platform-impl` |
| `Tree-selectChild` | Select Child Node | `TreeActions` | as `EditorRight` | MISSING | `platform/platform-impl` |
| `Tree-selectChildExtendSelection` | Extend Selection to Child Node | `TreeActions` | as `EditorRightWithSelection` | MISSING | `platform/platform-impl` |
| `Tree-selectFirst` | Select First Node | `TreeActions` | as `EditorTextStart` | MISSING | `platform/platform-impl` |
| `Tree-selectFirstExtendSelection` | Extend Selection to First Node | `TreeActions` | as `EditorTextStartWithSelection` | MISSING | `platform/platform-impl` |
| `Tree-selectLast` | Select Last Node | `TreeActions` | as `EditorTextEnd` | MISSING | `platform/platform-impl` |
| `Tree-selectLastExtendSelection` | Extend Selection to Last Node | `TreeActions` | as `EditorTextEndWithSelection` | MISSING | `platform/platform-impl` |
| `Tree-selectNext` | Select Next Node | `TreeActions` | as `EditorDown` | MISSING | `platform/platform-impl` |
| `Tree-selectNextExtendSelection` | Extend Selection with Next Node | `TreeActions` | as `EditorDownWithSelection` | MISSING | `platform/platform-impl` |
| `Tree-selectNextSibling` | Select Next Sibling Node | `TreeActions` |  | MISSING | `platform/platform-impl` |
| `Tree-selectParent` | Select Parent Node | `TreeActions` | as `EditorLeft` | MISSING | `platform/platform-impl` |
| `Tree-selectParentExtendSelection` | Extend Selection to Parent Node | `TreeActions` | as `EditorLeftWithSelection` | MISSING | `platform/platform-impl` |
| `Tree-selectParentNoCollapse` | Select Parent Node | `TreeActions` |  | MISSING | `platform/platform-impl` |
| `Tree-selectPrevious` | Select Previous Node | `TreeActions` | as `EditorUp` | MISSING | `platform/platform-impl` |
| `Tree-selectPreviousExtendSelection` | Extend Selection with Previous Node | `TreeActions` | as `EditorUpWithSelection` | MISSING | `platform/platform-impl` |
| `Tree-selectPreviousSibling` | Select Previous Sibling Node | `TreeActions` |  | MISSING | `platform/platform-impl` |
| `Tree-startEditing` | Edit Tree Node | `TreeActions` | `F2` | MISSING | `platform/platform-impl` |
| `UniversalFileChooser.FocusPath` | Focus Path Field |  | `control L` | MISSING | `platform/platform-api` |

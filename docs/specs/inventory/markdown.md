# Markdown

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (44)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `Markdown.ConfigurePandoc` | Configure Pandoc… | `Markdown.Tools` |  | MISSING | `plugins/markdown` |
| `Markdown.Diff.LivePreview` | Live Preview (Experimental) | `Diff.ViewerToolbarRight` |  | MISSING | `plugins/markdown` |
| `Markdown.Export` | Export Markdown File To… | `Markdown.Tools` |  | MISSING | `plugins/markdown` |
| `Markdown.Extensions.CleanupExternalFiles` | Clean Up Markdown Extensions External Files… |  |  | MISSING | `plugins/markdown` |
| `Markdown.GenerateTableOfContents` | Generate Table Of Contents | `Markdown.InsertGroup` |  | MISSING | `plugins/markdown` |
| `Markdown.ImportFromDocx` | Import Word Document… | `Markdown.Tools` |  | MISSING | `plugins/markdown` |
| `Markdown.Insert` | Insert… | `Markdown.EditorContextMenuGroup` | as `Generate` | MISSING | `plugins/markdown` |
| `Markdown.InsertEmptyTable` | Insert Table | `Markdown.InsertGroup` |  | MISSING | `plugins/markdown` |
| `Markdown.Layout.EditorOnly` | (set in code) |  |  | MISSING | `plugins/markdown` |
| `Markdown.Layout.LivePreview` | Live Preview (Experimental) |  |  | MISSING | `plugins/markdown` |
| `Markdown.NewFile` | Markdown File | `NewGroup` |  | MISSING | `plugins/markdown` |
| `Markdown.OpenDevtools` | Open Devtools Window For The Current Markdown Preview |  |  | MISSING | `plugins/markdown` |
| `Markdown.Preview.AdjustFontSize` | Adjust Font Size… | `Markdown.PreviewGroup` |  | MISSING | `plugins/markdown` |
| `Markdown.Preview.DecreaseFontSize` | Decrease Preview Font Size |  | as `CollapseAll` | MISSING | `plugins/markdown` |
| `Markdown.Preview.Find` | Find… | `Markdown.PreviewGroup` | as `Find` | MISSING | `plugins/markdown` |
| `Markdown.Preview.IncreaseFontSize` | Increase Preview Font Size |  | as `ExpandAll` | MISSING | `plugins/markdown` |
| `Markdown.Preview.ResetFontSize` | Reset Preview Font Size |  |  | MISSING | `plugins/markdown` |
| `Markdown.Styling.CreateLink` | Create Link | `Markdown.Toolbar.Floating`, `Markdown.InsertGroup` | `control shift U` | MISSING | `plugins/markdown` |
| `Markdown.Styling.CreateOrChangeList` | Create Or Change List | `Markdown.Toolbar.Floating` |  | MISSING | `plugins/markdown` |
| `Markdown.Styling.SetHeaderLevel` | Set Header Style | `Markdown.Toolbar.Floating` |  | MISSING | `plugins/markdown` |
| `Markdown.Table.InsertRow.InsertAbove` | Insert Row Above | `Markdown.TableRowActions`, `Markdown.TableContextMenuGroup` |  | MISSING | `plugins/markdown` |
| `Markdown.Table.InsertRow.InsertBelow` | Insert Row Below | `Markdown.TableRowActions`, `Markdown.TableContextMenuGroup` |  | MISSING | `plugins/markdown` |
| `Markdown.Table.InsertTableColumn.InsertAfter` | Insert Column Right | `Markdown.TableColumnActions`, `Markdown.TableContextMenuGroup` |  | MISSING | `plugins/markdown` |
| `Markdown.Table.InsertTableColumn.InsertBefore` | Insert Column Left | `Markdown.TableColumnActions`, `Markdown.TableContextMenuGroup` |  | MISSING | `plugins/markdown` |
| `Markdown.Table.RemoveCurrentColumn` | Remove Column | `Markdown.TableColumnActions`, `Markdown.TableContextMenuGroup` |  | MISSING | `plugins/markdown` |
| `Markdown.Table.RemoveCurrentRow` | Remove Row | `Markdown.TableRowActions`, `Markdown.TableContextMenuGroup` |  | MISSING | `plugins/markdown` |
| `Markdown.Table.SelectCurrentColumn.SelectContentCells` | Select Column Cells | `Markdown.TableColumnActions` |  | MISSING | `plugins/markdown` |
| `Markdown.Table.SelectRow` | Select Row | `Markdown.TableRowActions` |  | MISSING | `plugins/markdown` |
| `Markdown.Table.SetColumnAlignment.Center` | Align Center | `Markdown.Table.ColumnAlignmentActions` |  | MISSING | `plugins/markdown` |
| `Markdown.Table.SetColumnAlignment.Left` | Align Left | `Markdown.Table.ColumnAlignmentActions` |  | MISSING | `plugins/markdown` |
| `Markdown.Table.SetColumnAlignment.Right` | Align Right | `Markdown.Table.ColumnAlignmentActions` |  | MISSING | `plugins/markdown` |
| `Markdown.Table.SwapColumns.SwapWithLeftColumn` | Move Column Left | `Markdown.TableColumnActions`, `Markdown.TableContextMenuGroup` |  | MISSING | `plugins/markdown` |
| `Markdown.Table.SwapColumns.SwapWithRightColumn` | Move Column Right | `Markdown.TableColumnActions`, `Markdown.TableContextMenuGroup` |  | MISSING | `plugins/markdown` |
| `Markdown.Table.SwapRows.SwapWithAbove` | Move Row Up | `Markdown.TableRowActions` |  | MISSING | `plugins/markdown` |
| `Markdown.Table.SwapRows.SwapWithBelow` | Move Row Down | `Markdown.TableRowActions` |  | MISSING | `plugins/markdown` |
| `org.intellij.plugins.markdown.ui.actions.scrolling.AutoScrollAction` | Sync Scroll | `Markdown.Toolbar.Right` |  | MISSING | `plugins/markdown` |
| `org.intellij.plugins.markdown.ui.actions.styling.HeaderDownAction` | Decrease Header Level |  |  | MISSING | `plugins/markdown` |
| `org.intellij.plugins.markdown.ui.actions.styling.HeaderUpAction` | Increase Header Level |  |  | MISSING | `plugins/markdown` |
| `org.intellij.plugins.markdown.ui.actions.styling.InsertImageAction` | Insert Image | `Markdown.InsertGroup` | `control U` | MISSING | `plugins/markdown` |
| `org.intellij.plugins.markdown.ui.actions.styling.MarkdownIntroduceLinkReferenceAction` | Convert to Reference |  |  | MISSING | `plugins/markdown` |
| `org.intellij.plugins.markdown.ui.actions.styling.ToggleBoldAction` | Bold | `Markdown.Toolbar.Floating` | `control B` | MISSING | `plugins/markdown` |
| `org.intellij.plugins.markdown.ui.actions.styling.ToggleCodeSpanAction` | Code | `Markdown.Toolbar.Floating` | `control shift C` | MISSING | `plugins/markdown` |
| `org.intellij.plugins.markdown.ui.actions.styling.ToggleItalicAction` | Italic | `Markdown.Toolbar.Floating` | `control I` | MISSING | `plugins/markdown` |
| `org.intellij.plugins.markdown.ui.actions.styling.ToggleStrikethroughAction` | Strikethrough | `Markdown.Toolbar.Floating` | `control shift S` | MISSING | `plugins/markdown` |

## Settings pages (1)

| Id | Display name | Group | Parent | Scope | Spec |
|---|---|---|---|---|---|
| `Settings.Markdown` | Markdown | language |  | project | MISSING |

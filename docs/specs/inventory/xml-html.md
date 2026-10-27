# XML, HTML, CSS and Emmet

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (28)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `ConvertSchemaAction` | Convert Schema... | `ToolsXmlGroup` |  | MISSING | `xml/relaxng` |
| `Document2XSD` | Generate XML Document from XSD Schema… | `ToolsXmlGroupEx` |  | MISSING | `xml/xmlbeans` |
| `DomCollectionControl.Add` | Add | `DomCollectionControl` | `INSERT` | MISSING | `xml/dom-openapi` |
| `DomCollectionControl.Edit` | Edit | `DomCollectionControl` | as `EditSource` | MISSING | `xml/dom-openapi` |
| `DomCollectionControl.Remove` | Remove | `DomCollectionControl` | as `$Delete` | MISSING | `xml/dom-openapi` |
| `EmmetNextEditPoint` | Next Emmet Edit Point | `GoToEditPointGroup` | `alt shift CLOSE_BRACKET` | MISSING | `xml/emmet` |
| `EmmetPreview` | Emmet Preview | `Emmet` |  | MISSING | `xml/emmet` |
| `EmmetPreviousEditPoint` | Previous Emmet Edit Point | `GoToEditPointGroup` | `alt shift OPEN_BRACKET` | MISSING | `xml/emmet` |
| `EmmetUpdateTag` | Update tag with Emmet | `Emmet`, `EditorLangPopupMenu` |  | MISSING | `xml/emmet` |
| `EscapeEntities` | Encode XML/HTML Special Characters | `EditMenu` |  | MISSING | `xml/impl` |
| `GenerateDTD` | Generate DTD from XML File | `ToolsXmlGroupEx`, `XmlGenerateToolsGroup` |  | MISSING | `xml/xmlbeans` |
| `GenerateXmlTag` | XML Tag… | `GenerateGroup` |  | MISSING | `xml/impl` |
| `HtmlAddTableColumnAfter` | Add a New Column to the Table After the Current One | `GenerateGroup` |  | MISSING | `plugins/htmltools` |
| `HtmlAddTableColumnBefore` | Add a New Column to the Table Before the Current One | `GenerateGroup` |  | MISSING | `plugins/htmltools` |
| `HtmlTableCellNavigateDown` | Navigate to Next Table Row |  |  | MISSING | `plugins/htmltools` |
| `HtmlTableCellNavigateLeft` | Navigate to Previous Table Column |  |  | MISSING | `plugins/htmltools` |
| `HtmlTableCellNavigateRight` | Navigate to Next Table Column |  |  | MISSING | `plugins/htmltools` |
| `HtmlTableCellNavigateUp` | Navigate to Previous Table Row |  |  | MISSING | `plugins/htmltools` |
| `NewHtmlFile` | HTML File | `NewXml` |  | MISSING | `xml/impl` |
| `SurroundWithEmmet` | Surround with Emmet | `Emmet` |  | MISSING | `xml/emmet` |
| `SurroundWithTagTemplate` | Surround with Tag |  |  | MISSING | `xml/emmet` |
| `ValidateXml` | Validate | `ToolsXmlGroupEx`, `EditorPopupMenu.Run`, `ProjectViewPopupMenuRefactoringGroup` (+1) |  | MISSING | `xml/xmlbeans` |
| `WebBrowser` | Show Web Browser |  |  | MISSING | `plugins/jcef` |
| `WebOpenInAction` | Web Preview… |  | `alt F2` | MISSING | `plugins/htmltools` |
| `XPathView.Actions.Evaluate` | Evaluate XPath… | `XPathView.MainMenu.Search`, `XPathView.EditorPopup` | `control alt X, E` | MISSING | `plugins/xpath` |
| `XPathView.Actions.FindByExpression` | Find by XPath… | `XPathView.MainMenu.Search` | `control alt X, F` | MISSING | `plugins/xpath` |
| `XPathView.Actions.ShowPath` | Show Unique XPath | `CodeEditorViewGroup`, `XPathView.EditorPopup` | `control alt X, P` | MISSING | `plugins/xpath` |
| `XSD2Document` | Generate XSD Schema from XML File… | `ToolsXmlGroupEx` |  | MISSING | `xml/xmlbeans` |

## Settings pages (5)

| Id | Display name | Group | Parent | Scope | Spec |
|---|---|---|---|---|---|
| `xpath.settings` | XPath Viewer | tools |  | application | MISSING |
| `settings.xslt` | XSLT | language |  | application | MISSING |
| `reference.idesettings.emmet` | Emmet | editor |  | application | MISSING |
| `xslt.associations` | XSLT File Associations | language |  | project | MISSING |
| `preferences.externalResources` | Schemas and DTDs | language |  | project | MISSING |

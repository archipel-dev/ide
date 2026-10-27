# Search Everywhere, Go to Action, Run Anything

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (22)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `GotoAction` | Find Action… | `HelpMenu` | `control shift A` | MISSING | `platform/lang-impl` |
| `GotoClass` | Go to Class… | `GoToTargetEx` | `control N` | MISSING | `platform/lang-impl` |
| `GotoFile` | Go to File… | `GoToTargetEx`, `EditorTabsEntryPoint` | `control shift N` | MISSING | `platform/lang-impl` |
| `GotoRelated` | Related Symbol… | `GoToCodeGroup`, `EditorPopupMenu.GoTo` | `control alt HOME` | MISSING | `platform/lang-impl` |
| `GotoSymbol` | Go to Symbol… | `GoToTargetEx` | `control shift alt N` | MISSING | `platform/lang-impl` |
| `Se.IndexFiles` | Index Files for Lucene |  |  | MISSING | `plugins/search-everywhere-lucene` |
| `SeNewSearchToggleAction` | Use Split Search Everywhere |  |  | MISSING | `platform/searchEverywhere` |
| `SeSetJevKeyAction` | Set Jev API Key for Search Everywhere Ranker |  |  | MISSING | `platform/searchEverywhere` |
| `SeSetOpenAiKeyAction` | Set OpenAI API Key for Search Everywhere Ranker |  |  | MISSING | `platform/searchEverywhere` |
| `SearchEverywhere` | Search Everywhere | `NavBarToolBar`, `GoToMenu`, `MainToolBar` (+1) | `shift SHIFT dblClick` | codeinsight/06-completion.md CI-CMP-051 | `platform/lang-impl` |
| `SearchEverywhere.CompleteCommand` | Complete Command | `SearchEverywhereActions` | `TAB` | MISSING | `platform/platform-api` |
| `SearchEverywhere.NavigateToNextGroup` | Next Group Element | `SearchEverywhereActions` | `PAGE_DOWN`, `control DOWN` | MISSING | `platform/platform-api` |
| `SearchEverywhere.NavigateToPrevGroup` | Prev Group Element | `SearchEverywhereActions` | `PAGE_UP`, `control UP` | MISSING | `platform/platform-api` |
| `SearchEverywhere.NextTab` | Next Tab | `SearchEverywhereActions` | `TAB` | MISSING | `platform/platform-api` |
| `SearchEverywhere.PrevTab` | Prev Tab | `SearchEverywhereActions` | `shift TAB` | MISSING | `platform/platform-api` |
| `SearchEverywhere.SelectItem` | Select Item | `SearchEverywhereActions` | `Enter` | MISSING | `platform/platform-api` |
| `SearchEverywhereComponent` | Search Everywhere Component |  |  | MISSING | `platform/searchEverywhere` |
| `SearchEverywhereNewToolbarAction` | Search Everywhere | `LeftToolbarSideGroup` |  | codeinsight/06-completion.md CI-CMP-051 | `platform/lang-impl` |
| `ToggleSearchEverywhereMlSorting` | ML Sorting in Search Everywhere |  |  | MISSING | `plugins/search-everywhere-ml` |
| `com.intellij.searchEverywhereMl.ranking.core.actions.OpenFeaturesInScratchFileAction` | Show Search Everywhere Elements' ML Features | `MaintenanceGroup` | `shift ctrl alt 1` | MISSING | `plugins/search-everywhere-ml` |
| `com.intellij.searchEverywhereMl.typos.actions.CorpusToFileAction` | Corpus for the Current Tab to File | `MaintenanceGroup` |  | MISSING | `plugins/search-everywhere-ml` |
| `com.intellij.searchEverywhereMl.typos.actions.DumpDictionaryToFileAction` | Dump Dictionary for the Current Tab to File | `MaintenanceGroup` |  | MISSING | `plugins/search-everywhere-ml` |

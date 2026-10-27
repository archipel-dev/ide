# Languages and per-language extension points

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Languages with a parser definition (43)

Cells are the number of registrations for the language id (empty = none found in descriptors).

| Language | Plugin | syntaxHighlighterFactory | commenter | braceMatcher | foldingBuilder | formatter | psiStructureViewFactory | findUsagesProvider | refactoringSupport | documentationProvider | annotator | completion.contributor | ci.lineMarkerProvider | surroundDescriptor | namesValidator | ci.parameterInfo | importOptimizer | smartEnterProcessor | localInspection | intentionAction |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `RegExp` | RegExpSupport/resources | 1 |  | 1 |  |  |  |  |  | 1 | 2 | 1 |  | 1 |  |  |  |  | 15 | 3 |
| `JSRegexp` | RegExpSupport/resources | 1 |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
| `JSUnicodeRegexp` | RegExpSupport/resources |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
| `CommandLine` | commandInterface/resources |  |  |  |  |  |  |  |  | 1 |  |  |  |  |  |  |  |  |  |  |
| `SPI` | java/java-backend |  | 1 |  |  |  |  |  |  |  | 1 |  |  |  |  |  |  |  |  |  |
| `JShellLanguage` | java/java-psi-impl |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
| `JAVA` | java/java-psi-impl | 1 | 1 | 1 | 4 | 1 | 1 | 1 | 1 | 2 | 2 | 15 | 6 | 3 | 1 | 3 | 1 | 1 | 898 | 149 |
| `Manifest` | java/manifest | 1 |  |  |  |  |  |  |  |  | 1 | 1 |  |  |  |  |  |  | 2 |  |
| `JSON` | json/resources | 1 | 1 | 1 | 1 | 1 | 1 | 1 |  | 2 | 3 | 4 |  | 1 | 1 |  |  | 1 | 9 | 2 |
| `JSON5` | json/resources | 1 |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 1 |  |
| `JSON Lines` | json/resources |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
| `TEXT` | platform/core-impl | 1 |  | 1 | 1 |  |  |  |  |  | 3 | 7 |  |  |  |  |  |  | 9 |  |
| `AnalysisIgnore` | platform/lang-impl | 1 | 1 |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
| `IgnoreLang` | platform/vcs-impl | 1 |  |  |  |  |  |  |  |  |  | 2 |  |  |  |  |  |  | 1 |  |
| `ADLanguage` | plugins/devkit |  |  |  | 1 |  | 1 |  |  |  |  |  |  |  |  |  |  |  |  |  |
| `EditorConfig` | plugins/editorconfig | 1 | 1 | 1 | 1 | 1 |  |  |  |  | 2 | 2 |  |  |  |  |  |  | 28 | 2 |
| `DotEnv` | plugins/env-files-support | 1 | 1 |  | 1 |  |  |  |  |  |  | 1 |  |  |  |  |  |  | 11 |  |
| `GitIgnore` | plugins/git4idea |  | 1 | 1 |  |  |  |  |  |  |  |  | 1 |  |  |  |  |  |  |  |
| `GitExclude` | plugins/git4idea |  | 1 | 1 |  |  |  |  |  |  |  |  | 1 |  |  |  |  |  |  |  |
| `JSONPath` | plugins/jsonpath | 1 |  | 1 |  |  |  |  |  |  | 1 | 1 |  |  |  |  |  |  | 3 | 1 |
| `kotlin` | plugins/kotlin | 1 | 1 | 1 | 3 | 1 | 1 | 1 | 1 | 2 | 1 | 12 | 6 | 2 | 1 | 5 | 1 | 1 | 317 | 174 |
| `KND` | plugins/kotlin | 1 |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
| `Lombok.Config` | plugins/lombok | 1 | 1 |  |  |  |  |  |  |  |  | 1 |  |  |  |  |  |  |  |  |
| `Markdown` | plugins/markdown |  | 1 |  | 1 | 1 | 1 |  |  |  | 1 | 5 | 6 |  |  |  |  |  | 19 | 13 |
| `Mermaid` | plugins/mermaid |  | 1 | 1 | 2 | 1 |  |  |  |  | 5 | 1 |  |  |  |  |  |  | 5 |  |
| `IntegrationPerformanceTest` | plugins/performanceTesting | 1 |  |  |  |  |  |  |  |  | 1 | 1 |  |  |  |  |  |  |  |  |
| `Properties` | plugins/properties |  | 1 |  | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 3 |  |  | 1 |  |  |  | 11 | 2 |
| `ReST` | plugins/restructuredtext | 1 |  |  |  | 1 | 1 |  |  |  | 1 | 4 |  |  |  |  |  |  |  |  |
| `Shell Script` | plugins/sh |  | 1 | 1 | 1 |  | 1 | 1 |  | 1 | 1 | 4 |  |  |  |  |  |  | 1 | 2 |
| `JQL` | plugins/tasks | 1 |  | 1 |  |  |  |  |  |  | 1 | 1 |  |  |  |  |  |  |  |  |
| `YouTrack` | plugins/tasks |  |  |  |  |  |  |  |  |  |  | 1 |  |  |  |  |  |  |  |  |
| `TerminalOutput` | plugins/terminal |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
| `textmate` | plugins/textmate | 1 | 1 |  |  |  |  |  |  |  |  | 1 |  |  |  |  |  |  |  |  |
| `TOML` | plugins/toml |  | 1 | 1 | 1 | 1 |  | 1 |  |  | 1 | 3 |  |  |  |  |  |  | 2 | 1 |
| `XPath` | plugins/xpath | 1 |  | 1 |  |  |  | 1 | 1 | 1 | 2 | 1 |  |  | 1 | 1 |  |  | 5 |  |
| `XPath2` | plugins/xpath | 1 | 1 |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
| `yaml` | plugins/yaml | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 2 | 3 |  |  |  |  |  |  | 7 | 4 |
| `RELAX-NG` | xml/relaxng |  | 1 | 1 | 1 | 1 |  |  |  | 1 | 2 | 1 |  |  | 1 |  |  |  |  |  |
| `XsdRegExp` | xml/xml-psi-impl |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
| `DTD` | xml/xml-psi-impl | 1 | 1 |  | 1 |  | 1 | 1 |  | 2 |  | 1 |  |  |  |  |  |  |  |  |
| `XML` | xml/xml-psi-impl | 1 | 1 |  | 1 | 2 | 1 | 1 | 1 | 10 | 8 | 18 | 2 | 1 |  | 1 | 2 | 1 | 91 | 20 |
| `HTML` | xml/xml-psi-impl | 1 | 1 |  | 1 | 1 | 1 | 1 |  | 1 |  |  | 2 | 1 |  |  |  | 1 | 6 |  |
| `XHTML` | xml/xml-psi-impl | 1 | 1 |  | 1 | 1 | 1 | 1 |  | 1 |  |  | 1 | 1 |  |  |  | 1 |  |  |

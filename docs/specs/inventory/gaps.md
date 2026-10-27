# Spec gaps ranked by user impact

Ranking for a developer who works in Java, Kotlin, Gradle/Maven and Git. Impact is a judgement from 1 (rare) to 10 (every session); the counts are mechanical. 
Items = enumerated registrations assigned to the area (actions, settings pages, tool windows, run configuration types, status bar widgets, language inspections and intentions where noted). 
"Specified" = named in an existing spec or described by a feature-level requirement; "Framework" = only the generic framework spec exists (inspections, intentions and a few action areas); "Missing" = no spec.

Rows marked **TOP 10** are the proposed next spec waves, in order. Ties on impact are broken by item count.

| Rank | Gap | Impact | Items | Specified | Framework | Missing | Proposed spec file(s) | Top 10 |
|---|---|---|---|---|---|---|---|---|
| 1 | Editor basics: typing, caret, selection, clipboard, undo, lookup keys | 10 | 149 | 2 | 0 | 147 | `editor/01-typing-and-caret.md, editor/02-clipboard-and-undo.md` | **TOP 10** |
| 2 | Refactorings (rename, move, extract, inline, change signature, safe delete, ...) | 10 | 53 | 2 | 0 | 51 | `refactoring/01-rename-and-move.md, refactoring/02-extract-introduce-inline.md, refactoring/03-change-signature-and-safe-delete.md` | **TOP 10** |
| 3 | Java inspections catalog (all groups) | 9 | 1075 | 58 | 1017 | 0 | `java/09-inspections-catalog-1.md ... (one file per group chunk, mirroring inspections-java-N.md)` | **TOP 10** |
| 4 | Kotlin: actions, inspections, intentions | 9 | 551 | 1 | 501 | 49 | `kotlin/01-psi-and-resolve.md, kotlin/02-inspections.md, kotlin/03-intentions-and-quickfixes.md` | **TOP 10** |
| 5 | Debugger: breakpoints, stepping, evaluate, variables view | 9 | 134 | 0 | 0 | 134 | `debugger/01-breakpoints-and-stepping.md, debugger/02-evaluate-and-variables.md` | **TOP 10** |
| 6 | Run configurations, run tool window, services | 9 | 115 | 0 | 0 | 115 | `execution/01-run-configurations.md, execution/02-run-tool-window-and-services.md` | **TOP 10** |
| 7 | Editing actions: comment, duplicate, move line, join, surround, smart enter, extend selection | 9 | 12 | 0 | 0 | 12 | `editor/03-editing-actions.md` | **TOP 10** |
| 8 | Git and VCS core: commit, changes, branches, shelve, annotate, push/pull | 8 | 288 | 2 | 0 | 286 | `vcs/01-changes-and-commit.md, vcs/02-git-operations.md, vcs/03-branches-and-log.md` | **TOP 10** |
| 9 | Gradle and Maven import, tasks and tool windows; build and compile actions | 8 | 131 | 3 | 0 | 128 | `build/01-gradle.md, build/02-maven.md, build/03-compile-and-artifacts.md` | **TOP 10** |
| 10 | Find and Replace (in file, in path, structural search) | 8 | 28 | 4 | 0 | 24 | `search/01-find-replace-in-file.md, search/02-find-in-files.md` | **TOP 10** |
| 11 | Project view and project model UI (new file, mark root, structure dialogs) | 7 | 47 | 1 | 0 | 46 | `workbench/01-project-view.md` |  |
| 12 | Code style settings and the formatter (only hooks are specified) | 7 | 24 | 6 | 17 | 1 | `codeinsight/09-formatter-and-code-style.md` |  |
| 13 | Search Everywhere, Go to Action, Run Anything | 7 | 22 | 2 | 0 | 20 | `navigation/01-search-everywhere.md` |  |
| 14 | Test runner and coverage (JUnit, TestNG) | 7 | 14 | 0 | 0 | 14 | `execution/03-test-runner.md` |  |
| 15 | Java language actions (generate, module and compiler specific) | 6 | 251 | 1 | 152 | 98 | `java/10-generate-and-java-actions.md, java/11-intentions-catalog.md` |  |
| 16 | Live templates and surround templates | 6 | 162 | 0 | 0 | 162 | `editor/04-live-templates.md` |  |
| 17 | Diff and merge viewer | 6 | 63 | 1 | 0 | 62 | `vcs/04-diff-and-merge.md` |  |
| 18 | Code insight actions (docs, parameter info, generate menu, inline completion) | 6 | 59 | 25 | 34 | 0 | `codeinsight/11-documentation-and-parameter-info.md` |  |
| 19 | Code folding | 6 | 30 | 2 | 0 | 28 | `editor/06-folding.md` |  |
| 20 | Multi-caret and column selection | 6 | 13 | 0 | 0 | 13 | `editor/05-multi-caret.md` |  |
| 21 | XML, HTML, CSS and Emmet (language support, inspections, intentions) | 5 | 146 | 0 | 113 | 33 | `xml/01-xml-html-support.md` |  |
| 22 | Windows, tool windows, tabs, splits | 5 | 138 | 1 | 0 | 137 | `workbench/03-tool-windows-and-tabs.md` |  |
| 23 | Settings dialog, keymap, appearance, schemes | 5 | 107 | 4 | 0 | 103 | `workbench/04-settings-and-keymap.md` |  |
| 24 | File types registry and associations | 5 | 71 | 0 | 0 | 71 | `platform/09-file-types.md` |  |
| 25 | Terminal | 5 | 58 | 6 | 0 | 52 | `workbench/05-terminal.md` |  |
| 26 | Problems view, Inspect Code UI, scopes | 5 | 38 | 12 | 24 | 2 | `codeinsight/10-problems-view-and-analyze.md` |  |
| 27 | Structure view and file structure popup | 5 | 2 | 1 | 0 | 1 | `workbench/02-structure-view.md` |  |
| 28 | Bookmarks | 4 | 102 | 0 | 0 | 102 | `workbench/06-bookmarks.md` |  |
| 29 | Markdown | 4 | 77 | 0 | 32 | 45 | `lang/02-markdown.md` |  |
| 30 | File operations (save, sync, reload, scratch, encoding) | 4 | 56 | 6 | 50 | 0 | `workbench/08-file-operations.md` |  |
| 31 | JSON, YAML, properties, TOML | 4 | 52 | 0 | 38 | 14 | `lang/01-json-yaml-properties-toml.md` |  |
| 32 | Settings pages not claimed by an area, tool windows and status bar widgets | 4 | 47 | 0 | 0 | 47 | `workbench/13-tool-windows-and-status-bar.md` |  |
| 33 | Welcome screen, new project, open and recent projects | 4 | 24 | 2 | 0 | 22 | `workbench/09-welcome-and-projects.md` |  |
| 34 | Local history | 4 | 10 | 0 | 0 | 10 | `vcs/05-local-history.md` |  |
| 35 | TODO tool window | 4 | 6 | 0 | 0 | 6 | `workbench/07-todo.md` |  |
| 36 | Other-language inspections and intentions (UAST, platform) | 3 | 113 | 1 | 112 | 0 | `codeinsight/13-platform-inspections-and-intentions.md` |  |
| 37 | Generic UI component key handling (list, tree, table, popup) | 3 | 91 | 0 | 0 | 91 | `workbench/12-ui-component-keys.md` |  |
| 38 | GitHub and GitLab integration | 3 | 51 | 0 | 0 | 51 | `vcs/06-git-hosting.md` |  |
| 39 | TextMate, EditorConfig, regexp, env files | 3 | 37 | 0 | 19 | 18 | `lang/04-misc-languages.md` |  |
| 40 | Plugin management and updates | 3 | 12 | 0 | 0 | 12 | `workbench/11-plugins.md` |  |
| 41 | Spelling and grammar | 3 | 3 | 0 | 0 | 3 | `codeinsight/12-spellchecking.md` |  |
| 42 | Tasks and contexts | 2 | 12 | 0 | 0 | 12 | `workbench/10-tasks-and-contexts.md` |  |
| 43 | Shell script support | 2 | 8 | 0 | 3 | 5 | `lang/03-shell-script.md` |  |
| 44 | AI/MCP/tips/feedback, internal and plugin-development tools | 1 | 337 | 5 | 0 | 332 | `out of scope for the first releases` |  |

## Notes on the top 10

1. **Editor basics: typing, caret, selection, clipboard, undo, lookup keys** (149 items, 147 missing). Proposed: `editor/01-typing-and-caret.md, editor/02-clipboard-and-undo.md`. Every keystroke goes through these actions; documents are specified, editor actions are not. Item lists: see the area files linked from `README.md`.
2. **Refactorings (rename, move, extract, inline, change signature, safe delete, ...)** (53 items, 51 missing). Proposed: `refactoring/01-rename-and-move.md, refactoring/02-extract-introduce-inline.md, refactoring/03-change-signature-and-safe-delete.md`. Refactoring is the headline IDE feature; no spec touches it. Item lists: see the area files linked from `README.md`.
3. **Java inspections catalog (all groups)** (1075 items, 0 missing). Proposed: `java/09-inspections-catalog-1.md ... (one file per group chunk, mirroring inspections-java-N.md)`. JAVA-INS covers a few dozen default-on checks; the catalog is far larger. Item lists: see the area files linked from `README.md`.
4. **Kotlin: actions, inspections, intentions** (551 items, 49 missing). Proposed: `kotlin/01-psi-and-resolve.md, kotlin/02-inspections.md, kotlin/03-intentions-and-quickfixes.md`. Kotlin is a first-class target and has no spec at all. Item lists: see the area files linked from `README.md`.
5. **Debugger: breakpoints, stepping, evaluate, variables view** (134 items, 134 missing). Proposed: `debugger/01-breakpoints-and-stepping.md, debugger/02-evaluate-and-variables.md`. Largest untouched interactive feature. Item lists: see the area files linked from `README.md`.
6. **Run configurations, run tool window, services** (115 items, 115 missing). Proposed: `execution/01-run-configurations.md, execution/02-run-tool-window-and-services.md`. Run/Debug is used in every session. Item lists: see the area files linked from `README.md`.
7. **Editing actions: comment, duplicate, move line, join, surround, smart enter, extend selection** (12 items, 12 missing). Proposed: `editor/03-editing-actions.md`. Small count, very high frequency. Item lists: see the area files linked from `README.md`.
8. **Git and VCS core: commit, changes, branches, shelve, annotate, push/pull** (288 items, 286 missing). Proposed: `vcs/01-changes-and-commit.md, vcs/02-git-operations.md, vcs/03-branches-and-log.md`. Git is in the required stack; whole family unspecified. Item lists: see the area files linked from `README.md`.
9. **Gradle and Maven import, tasks and tool windows; build and compile actions** (131 items, 128 missing). Proposed: `build/01-gradle.md, build/02-maven.md, build/03-compile-and-artifacts.md`. Project model depends on them. Item lists: see the area files linked from `README.md`.
10. **Find and Replace (in file, in path, structural search)** (28 items, 24 missing). Proposed: `search/01-find-replace-in-file.md, search/02-find-in-files.md`. Find Usages is specified; text search is not. Item lists: see the area files linked from `README.md`.

## How the proposed file names were chosen

New spec directories follow the existing pattern (`<area>/NN-topic.md` with a requirement ID prefix). Existing directories are reused where the topic is a natural extension (`codeinsight/`, `java/`, `platform/`). 
Nothing here is scheduled work: the list only says where specs would go.

# IntelliJ IDEA Community feature inventory

A machine-extracted list of what IntelliJ IDEA Community registers (actions, settings pages, tool windows, run configuration types, inspections, intentions, file types, live templates, status bar widgets), grouped by user-visible feature area, with the spec that covers each item or `MISSING`. 
It contains facts only (ids, names, user-visible texts, defaults, default shortcuts); behavior is specified in the other spec sets. Source reading follows `docs/specs/README.md`; sources are cited as `ic: <path>`.

## Method

1. Basis: the intellij-community repository at commit `e863da0b3f34c53a7c2e39763e3129215121d108` (the existing specs cite tag `idea/262.10968.63`; registrations drift slowly but counts may differ by a few percent).
2. Every non-test XML file under the platform, java, xml, json, uast, spellchecker, regexp and `plugins/` trees that is a plugin descriptor (`<idea-plugin>` root) was parsed: `<actions>` (actions, groups, `add-to-group`, `reference`, keyboard shortcuts) and `<extensions>` (all extension points). The default keymap `platform/platform-resources/src/keymaps/$default.xml` supplies default shortcuts. ic: platform/platform-resources/src/keymaps/$default.xml
3. User-visible texts were resolved from message bundles (`messages/*Bundle*.properties`): `action.<id>.text`, inspection `key`/`groupKey`, configurable `key`, intention category keys, tool window stripe titles. Mnemonics were removed and `{0}` placeholders shown as an ellipsis. Texts that are computed in code show `(set in code)`.
4. Intention and inspection descriptions are the first sentence of the shipped `intentionDescriptions/*/description.html` and `inspectionDescriptions/*.html` resources (short user-visible facts).
5. Run configuration type ids and names were read from the type classes where they are literal; otherwise `(see class)`.
6. Each item was assigned to one area by rules on the owning plugin path, the action id, and group ancestry (heuristic, see "Limits").
7. The `Spec` column was filled by searching `docs/specs/*/*.md` for the item id (or, for actions, the exact text) and reporting the file and nearest requirement ID; a small curated table maps feature-level spec sets onto action families (completion, navigation, optimize imports and reformat, inspection UI, auto-import).

The extraction scripts are deliberately not in the repository (only facts are). To refresh, rerun the same pipeline against a new IntelliJ commit: download descriptors and bundles, parse as above, regenerate these files, and compare the totals in this README. A refresh is due when the specs move to a new IntelliJ tag.

## Scope

In scope: platform, Java, Kotlin, Gradle, Maven, Git (and GitHub/GitLab), terminal, Markdown, XML/HTML, JSON/YAML/properties/TOML, shell script, TextMate, EditorConfig, spelling/grammar, tasks, and the generic workbench. 
Out of scope (counted but not itemised), with reasons:

| Family | Reason | Registrations skipped |
|---|---|---|
| Groovy language support | Groovy language support (Groovy-specific) | 18 actions, 2 configurables, 5 filetypes, 136 inspections, 76 intentions, 1 languages, 4 ltcontext, 1 runconfigs |
| Database data grid | Database data grid (shared with database IDEs, not Java/Kotlin scope) | 106 actions, 2 configurables, 1 filetypes, 2 statusbar |
| Swing GUI designer | Swing GUI designer (legacy form editor) | 32 actions, 1 configurables, 1 filetypes, 10 inspections |
| Mercurial integration | Mercurial integration (Git is the in-scope VCS) | 28 actions, 1 configurables, 1 filetypes, 1 languages, 3 statusbar |
| Image viewer and editor | Image viewer and editor | 24 actions, 2 filetypes, 1 languages |
| Subversion integration | Subversion integration (Git is the in-scope VCS) | 22 actions, 1 configurables |
| JavaFX support | JavaFX support (niche framework) | 3 actions, 1 configurables, 1 filetypes, 7 inspections, 3 intentions |
| Notebook visualization | Notebook visualization (data science) | 8 actions |
| Jupyter/MPS/Fleet specific | Jupyter/MPS/Fleet specific | 1 filetypes |

Families excluded by path before parsing (no descriptors read): `python/` (PyCharm and Python plugins), Android and `android*` plugins, `libraries/` (bundled library wrappers, no features), `build/`, `tools/`, `native/`, `jps/` (build engine), and all test and test-data trees. Descriptors that exist only to wire modules (no actions or user-visible extensions) contribute nothing.

## Totals (in-scope items)

| Category | Items | Named or described in a spec | Framework only | Missing | Specified share |
|---|---|---|---|---|---|
| Actions | 2422 | 105 | 211 | 2106 | 4.3% |
| Settings pages (configurables) | 103 | 8 | 0 | 95 | 7.8% |
| Tool windows | 23 | 0 | 0 | 23 | 0.0% |
| Run configuration types | 16 | 0 | 0 | 16 | 0.0% |
| Inspections | 1631 | 59 | 1572 | 0 | 3.6% |
| Intentions | 415 | 0 | 415 | 0 | 0.0% |
| File types | 71 | 0 | 0 | 71 | 0.0% |
| Live templates | 102 | 0 | 0 | 102 | 0.0% |
| Live template contexts | 49 | 0 | 0 | 49 | 0.0% |
| Status bar widgets | 24 | 0 | 0 | 24 | 0.0% |
| Keymaps | 26 | 0 | 0 | 26 | 0.0% |
| **All** | 4882 | 172 | 2198 | 2512 | 3.5% |

Languages with a parser definition: 43 (see `languages.md`). Extension-point census: `extension-points.md`.

## Areas

Items per area (actions, settings pages, tool windows, run configuration types, status bar widgets, live templates, keymaps; language inspections and intentions are in their own files).

| Area | File | Actions | Settings | Tool windows | Run configs | Other | Specified | Framework | Missing |
|---|---|---|---|---|---|---|---|---|---|
| Editor basics (caret, selection, scrolling, typing, clipboard, undo) | [editor-basics.md](editor-basics.md) | 149 | 0 | 0 | 0 | 0 | 2 | 0 | 147 |
| Editing actions (comment, duplicate, move, join, surround, case, smart enter) | [editing-actions.md](editing-actions.md) | 12 | 0 | 0 | 0 | 0 | 0 | 0 | 12 |
| Multi-caret and column selection | [multi-caret.md](multi-caret.md) | 13 | 0 | 0 | 0 | 0 | 0 | 0 | 13 |
| Code folding | [folding.md](folding.md) | 30 | 0 | 0 | 0 | 0 | 2 | 0 | 28 |
| Live templates and file templates | [live-templates.md](live-templates.md) | 8 | 3 | 0 | 0 | 151 | 0 | 0 | 162 |
| Code style, reformat and imports | [code-style.md](code-style.md) | 22 | 2 | 0 | 0 | 0 | 6 | 17 | 1 |
| Code insight actions (completion, documentation, parameter info, generate, intentions) | [code-insight.md](code-insight.md) | 57 | 2 | 0 | 0 | 0 | 25 | 34 | 0 |
| Refactorings | [refactorings.md](refactorings.md) | 53 | 0 | 0 | 0 | 0 | 2 | 0 | 51 |
| Find and Replace | [find-replace.md](find-replace.md) | 28 | 0 | 0 | 0 | 0 | 4 | 0 | 24 |
| Find Usages and usage views | [find-usages.md](find-usages.md) | 25 | 0 | 0 | 0 | 0 | 7 | 18 | 0 |
| Search Everywhere, Go to Action, Run Anything | [search-everywhere.md](search-everywhere.md) | 22 | 0 | 0 | 0 | 0 | 2 | 0 | 20 |
| Navigation (go to, back/forward, recent files, navigation bar) | [navigation.md](navigation.md) | 85 | 0 | 0 | 0 | 0 | 17 | 68 | 0 |
| Project view and project model UI | [project-view.md](project-view.md) | 44 | 1 | 2 | 0 | 0 | 1 | 0 | 46 |
| Structure view and file structure popup | [structure-view.md](structure-view.md) | 1 | 0 | 1 | 0 | 0 | 1 | 0 | 1 |
| Problems view, Analyze and inspection UI | [problems-analyze.md](problems-analyze.md) | 35 | 2 | 1 | 0 | 0 | 12 | 24 | 2 |
| TODO tool window | [todo.md](todo.md) | 4 | 1 | 1 | 0 | 0 | 0 | 0 | 6 |
| Bookmarks | [bookmarks.md](bookmarks.md) | 101 | 0 | 1 | 0 | 0 | 0 | 0 | 102 |
| Run and Debug configurations, services, run tool window | [run-debug.md](run-debug.md) | 101 | 5 | 0 | 8 | 1 | 0 | 0 | 115 |
| Test runner and coverage | [test-runner.md](test-runner.md) | 10 | 1 | 1 | 2 | 0 | 0 | 0 | 14 |
| Debugger | [debugger.md](debugger.md) | 133 | 1 | 0 | 0 | 0 | 0 | 0 | 134 |
| Build, compile and artifacts | [build.md](build.md) | 19 | 5 | 1 | 1 | 0 | 1 | 0 | 25 |
| Gradle and external systems | [gradle.md](gradle.md) | 44 | 2 | 1 | 1 | 0 | 1 | 0 | 47 |
| Maven | [maven.md](maven.md) | 55 | 1 | 0 | 1 | 0 | 1 | 0 | 56 |
| Version control core (commit, changelists, shelve, annotate, history) | [vcs-core.md](vcs-core.md) | 114 | 1 | 2 | 0 | 1 | 2 | 0 | 116 |
| VCS log and branches (generic) | [vcs-log.md](vcs-log.md) | 37 | 1 | 0 | 0 | 0 | 0 | 0 | 38 |
| Git | [git.md](git.md) | 129 | 1 | 0 | 0 | 2 | 0 | 0 | 132 |
| GitHub and GitLab | [git-hosting.md](git-hosting.md) | 47 | 2 | 2 | 0 | 0 | 0 | 0 | 51 |
| Local history | [local-history.md](local-history.md) | 10 | 0 | 0 | 0 | 0 | 0 | 0 | 10 |
| Diff and merge viewer | [diff-viewer.md](diff-viewer.md) | 62 | 1 | 0 | 0 | 0 | 1 | 0 | 62 |
| Terminal | [terminal.md](terminal.md) | 56 | 1 | 1 | 0 | 0 | 6 | 0 | 52 |
| Settings, keymap, appearance and schemes | [settings-keymap.md](settings-keymap.md) | 24 | 39 | 4 | 0 | 40 | 4 | 0 | 103 |
| Plugin management, updates and licensing | [plugins-mgmt.md](plugins-mgmt.md) | 12 | 0 | 0 | 0 | 0 | 0 | 0 | 12 |
| Welcome screen, new project, open and recent projects | [welcome-project.md](welcome-project.md) | 24 | 0 | 0 | 0 | 0 | 2 | 0 | 22 |
| Windows, tool windows, tabs, splits and view modes | [window-ui.md](window-ui.md) | 137 | 0 | 1 | 0 | 0 | 1 | 0 | 137 |
| File operations (save, sync, reload, scratch files, VFS) | [file-ops.md](file-ops.md) | 56 | 0 | 0 | 0 | 0 | 6 | 50 | 0 |
| Kotlin | [kotlin.md](kotlin.md) | 45 | 3 | 0 | 2 | 0 | 1 | 0 | 49 |
| Java language support (generate, code, compiler specific) | [java-lang.md](java-lang.md) | 92 | 7 | 0 | 0 | 0 | 1 | 0 | 98 |
| XML, HTML, CSS and Emmet | [xml-html.md](xml-html.md) | 28 | 5 | 0 | 0 | 0 | 0 | 0 | 33 |
| JSON, YAML, properties and TOML | [json-yaml-props-toml.md](json-yaml-props-toml.md) | 11 | 2 | 0 | 0 | 1 | 0 | 0 | 14 |
| Markdown | [markdown.md](markdown.md) | 44 | 1 | 0 | 0 | 0 | 0 | 0 | 45 |
| Shell script | [shell-script.md](shell-script.md) | 4 | 0 | 0 | 1 | 0 | 0 | 0 | 5 |
| Other languages and text support (TextMate, EditorConfig, regexp, env files) | [languages-misc.md](languages-misc.md) | 13 | 5 | 0 | 0 | 0 | 0 | 0 | 18 |
| Spelling and grammar | [spell-grammar.md](spell-grammar.md) | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 3 |
| AI, MCP, tips, feedback and statistics | [ai-misc.md](ai-misc.md) | 45 | 2 | 0 | 0 | 1 | 1 | 0 | 47 |
| Plugin development, internal and diagnostic tools | [devkit-internal.md](devkit-internal.md) | 278 | 3 | 4 | 0 | 4 | 4 | 0 | 285 |
| Generic UI component actions (list, tree, table, popup, scroll-pane navigation) | [ui-widgets.md](ui-widgets.md) | 91 | 0 | 0 | 0 | 0 | 0 | 0 | 91 |
| Tasks and contexts | [tasks-contexts.md](tasks-contexts.md) | 12 | 0 | 0 | 0 | 0 | 0 | 0 | 12 |

## Inspections and intentions (by language)

| Language | Inspections file(s) | Inspections | Intentions file | Intentions |
|---|---|---|---|---|
| devkit-internal | [inspections-devkit-internal.md](inspections-devkit-internal.md) | 3 |  | 0 |
| java | [inspections-java-1.md](inspections-java-1.md), [inspections-java-2.md](inspections-java-2.md), [inspections-java-3.md](inspections-java-3.md), [inspections-java-4.md](inspections-java-4.md), [inspections-java-5.md](inspections-java-5.md), [inspections-java-6.md](inspections-java-6.md), [inspections-java-7.md](inspections-java-7.md), [inspections-java-8.md](inspections-java-8.md), [inspections-java-9.md](inspections-java-9.md), [inspections-java-10.md](inspections-java-10.md) | 1075 | [intentions-java.md](intentions-java.md) | 152 |
| json-yaml-props-toml | [inspections-json-yaml-props-toml.md](inspections-json-yaml-props-toml.md) | 29 | [intentions-json-yaml-props-toml.md](intentions-json-yaml-props-toml.md) | 9 |
| kotlin | [inspections-kotlin-1.md](inspections-kotlin-1.md), [inspections-kotlin-2.md](inspections-kotlin-2.md), [inspections-kotlin-3.md](inspections-kotlin-3.md), [inspections-kotlin-4.md](inspections-kotlin-4.md) | 325 | [intentions-kotlin.md](intentions-kotlin.md) | 176 |
| languages-misc | [inspections-languages-misc.md](inspections-languages-misc.md) | 16 | [intentions-languages-misc.md](intentions-languages-misc.md) | 3 |
| markdown | [inspections-markdown.md](inspections-markdown.md) | 19 | [intentions-markdown.md](intentions-markdown.md) | 13 |
| other | [inspections-other.md](inspections-other.md) | 70 | [intentions-other.md](intentions-other.md) | 40 |
| shell-script | [inspections-shell-script.md](inspections-shell-script.md) | 1 | [intentions-shell-script.md](intentions-shell-script.md) | 2 |
| xml-html | [inspections-xml-html.md](inspections-xml-html.md) | 93 | [intentions-xml-html.md](intentions-xml-html.md) | 20 |

## Other files

- [file-types.md](file-types.md): file types (registered plus platform defaults).
- [languages.md](languages.md): languages and which language-level extension points each registers.
- [extension-points.md](extension-points.md): registration counts per extension point.
- [gaps.md](gaps.md): areas ranked by user impact with the proposed spec file for each gap (top 10 marked).

## Limits

- Registrations in code (dynamic actions, actions created at runtime, tool windows added by code) are not visible in descriptors. Counts are a lower bound.
- Area assignment is heuristic: a few actions sit in a neighboring area. Wrong placement affects only which file lists an item, not totals.
- The clone used is sparse; descriptors were read from the repository tree at the commit above, so no descriptor is missing for that reason. Texts that live in non-English-only or generated bundles show `(set in code)`.
- `Specified` only means an existing spec names or describes the item. It does not judge completeness. `framework` marks inspections and intentions (generic mechanism specified in `codeinsight/03` and `04`), plus a few action areas where the framework spec is the only coverage.
- Inspection matching uses the short name; Java inspections described by display name in `java/07-inspections.md` may be undercounted.
- These files define no requirement IDs and have no requirement table header, so `cargo run -p xtask -- spec-coverage --check` is unaffected (verified when generated).


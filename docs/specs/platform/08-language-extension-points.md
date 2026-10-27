# Platform spec 08: language extension points (what a language plugin implements)

Status: specification (clean-room). ID prefix `PLAT-LANG`. Priorities: `MUST` / `SHOULD` / `LATER` as in `README.md`.

Supersedes and deepens section 1.3 (language plugin interface and extension registry) of `docs/analysis/code-intelligence-spec.md`
(decision D3 there: one `LanguageDef` carrying a typed registry of optional services). This document specifies, for each extension point,
its **purpose, when the platform calls it, and its contract**. Higher-level features (completion, inspections, refactoring, formatting, documentation) have
their own specs owned by other teams; this document only lists their extension points and the platform-side rules they share (section 5).

Sources studied: `ic: platform/core-api/src/com/intellij/lang/` (`Language.java`, `LanguageExtension.java`, `LanguageExtensionWithAny.java`, `ParserDefinition.java`, `PsiParser.java`,
`PsiBuilder.java`, `SyntaxTreeBuilder.java`, `Commenter.java`, `LanguageParserDefinitions.java`, `LanguageUtil.java`, `DependentLanguage.java`, `InjectableLanguage.java`, `MetaLanguage.java`,
`folding/FoldingBuilder.java`, `folding/FoldingBuilderEx.java`, `folding/LanguageFolding.java`, `injection/MultiHostInjector.java`), `ic: platform/core-api/src/com/intellij/lexer/Lexer.java`,
`ic: platform/core-api/src/com/intellij/openapi/fileTypes/` (`FileType.java`, `LanguageFileType.java`, `FileTypeRegistry.java`), `ic: platform/core-api/src/com/intellij/psi/` (`LanguageSubstitutor.java`,
`FileViewProviderFactory.java`, `PsiReferenceContributor.java`, `PsiReferenceProvider.java`, `PsiReferenceRegistrar.java`, `ElementManipulator.java`, `ElementDescriptionProvider.java`),
`ic: platform/editor-ui-api/src/com/intellij/openapi/fileTypes/SyntaxHighlighter(Factory).java`, `ic: platform/analysis-api/src/com/intellij/lang/` (`annotation/Annotator.java`, `annotation/AnnotationHolder.java`,
`annotation/ExternalAnnotator.java`, `annotation/HighlightSeverity.java`, `PairedBraceMatcher.java`, `refactoring/NamesValidator.java`, `LanguageAnnotators.java`, `LanguageBraceMatching.java`, `LanguageNamesValidation.java`),
`ic: platform/indexing-api/src/com/intellij/lang/findUsages/FindUsagesProvider.java`, `ic: platform/lang-api/src/com/intellij/codeInsight/daemon/LineMarkerProvider.java`,
`ic: platform/lang-api/src/com/intellij/codeInsight/editorActions/` (`TypedHandlerDelegate.java`, `ExtendWordSelectionHandler.java`), `ic: platform/lang-impl/src/com/intellij/codeInsight/editorActions/QuoteHandler.java`,
`ic: platform/code-style-api/src/com/intellij/lang/ImportOptimizer.java`, `ic: platform/lang-api/src/com/intellij/navigation/ChooseByNameContributor.java`,
`ic: platform/analysis-api/src/com/intellij/codeInsight/navigation/actions/GotoDeclarationHandler.java`, `ic: java/java-psi-api/src/com/intellij/psi/PsiElementFinder.java`,
`ic: java/java-psi-api/src/com/intellij/psi/augment/PsiAugmentProvider.java`, `ic: platform/analysis-impl/src/com/intellij/psi/impl/file/impl/ResolveScopeManagerImpl.java`.

## 1. Language identity and extension lookup

### 1.1 Language

A **language** is a singleton with a unique string id, optional MIME types, an optional **base language** (a dialect inherits from it), a display name, a case-sensitivity flag, and
a list of its dialects. A special **"any language"** value exists, used as the key for extensions that apply to every language. Languages are registered when their plugin loads and can be unregistered when it unloads.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-LANG-001 | MUST | A language has exactly one instance per process, found by id or by its type; ids are unique; MIME types are looked up as a multi-map. Registering a second language with an existing id is rejected. | Two registrations with id `JAVA`: second fails with both plugin names in the message. |
| PLAT-LANG-002 | MUST | Dialect relation: `is(other)` = same language; `is_kind_of(other)` = same or a (transitive) dialect of `other`; `dialects()` direct; `transitive_dialects()` all. A dialect's base language is used as a fallback for extension lookup (1.2). | `KotlinScript.is_kind_of(Kotlin)` true; `Kotlin.is_kind_of(KotlinScript)` false. |
| PLAT-LANG-003 | MUST | `is_case_sensitive()`: delegates to the base language when present, else false unless overridden. Used by name matching, completion and rename conflict detection. | Java: true; SQL dialect without override: false. |
| PLAT-LANG-004 | SHOULD | Marker categories: *dependent* (not user-visible, e.g. languages of lazy-parseable nodes or substitutor dialects), *injectable* (may be offered for injection), *template* (with a data language), *meta* (matches several languages by predicate). The UI hides dependent languages from pickers. | A language marked dependent does not appear in the "inject language" list. |
| PLAT-LANG-005 | MUST | A language and its parts are **removable at runtime** (plugin unload): all caches keyed by the language (extension caches, stub versions, element types) are cleared and element types of the plugin become tombstones. | LATER if no dynamic plugins in the first release; the design must not assume the registry is immutable after startup. |

### 1.2 Extension lookup by language (the registry contract)

Every language-keyed extension point is a **language extension**: a collection keyed by language id. Lookup rules:

* **Single-instance lookup** (`for_language(L)`): look for extensions registered for `L`; if none, for `L`'s base language, then its base, and so on; the **first extension in registration/loading order** at the first level that has any wins; if none at any level, a **default implementation** if the extension point declares one, else *none*.
* **All-instances lookup** (`all_for_language(L)`): all extensions registered for `L`, then those for its base languages (outermost last). A variant adds the default implementation, another adds those registered for the **any-language key**.
* Results are cached per language and invalidated when contributions are added or removed.
* A **meta-language** key matches every language its predicate accepts.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-LANG-010 | MUST | Provide a generic language-extension collection with the lookups above (single, all, all-with-default, all-or-any), caching, invalidation, and explicit add/remove for tests. | Unit tests for each lookup on a 3-level dialect chain. |
| PLAT-LANG-011 | MUST | Single-instance lookups are *deterministic*: ordering among extensions registered for the same language follows an explicit **loading order** (first, last, before id, after id; the kernel already has this, `docs/architecture/platform-kernel.md`). | Register two for one language with `first`/`last`: `first` wins. |
| PLAT-LANG-012 | MUST | An extension point declares which lookup its callers use. Parser definition, commenter, brace matcher, names validator, structure view factory, documentation provider, find-usages provider (uses *all*; first non-null per method) use *single*. Annotators, line markers, folding builders (combined into one composite), completion contributors, inspections, intentions, typed handlers, injectors use *all* (and *or any* where noted). | Review table in section 4 matches the registry code. |
| PLAT-LANG-013 | MUST | Extensions are **application-level shared instances**: they are created lazily once, shared by all projects and threads, and must be stateless or thread-safe. Per-project data is obtained from the arguments, not stored. | A stress test calls one extension instance from 16 threads. |
| PLAT-LANG-014 | MUST | Language-extension code must not assume that the language it receives is the language it was registered for (a dialect passes through its base language's extensions). | A base-language folding builder handles a dialect's tree. |

## 2. Core language definition

### 2.1 File type and language file type

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-LANG-020 | MUST | **Purpose**: classify files; **called**: whenever a file's type is asked for (PLAT-VFS-071); **contract**: singleton, unique name, declared *language* in the registration so the language can answer "my file type" without loading the type class; if the declared language disagrees with the type's own, registration is an error. Exactly one non-secondary file type per language; others are marked secondary. | See PLAT-VFS-070, 079. |
| PLAT-LANG-021 | MUST | A file type may declare: extensions, exact file names, wildcard patterns, hashbang lines, default extension, binary flag, read-only flag, a charset (fixed or content-extracted: the content-extraction hook receives the already-decoded text and returns a charset or none; callers skip file types that do not override it). | An XML file type extracts the prolog encoding. |
| PLAT-LANG-022 | SHOULD | A **file type factory** or declarative registration is lazy: the type class is loaded only when a file of that type is first seen. | Startup does not load the Rust language until a `.rs` file is touched. |

### 2.2 Parser definition, lexer, tree builder

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-LANG-030 | MUST | **ParserDefinition**: *purpose* turn text into a tree and elements. *Called*: when a file's tree is first built or rebuilt (full parse), when an incremental reparse falls back to a full parse, for stub building, and for indexers that need a tree. *Contract*: `create_lexer(project)` returns a **new** lexer for whole-file lexing (it need not support restart); `create_parser(project)`; `file_node_type()`; whitespace token set (just the shared whitespace type unless needed); comment token set (root types only for composite comments; also used for TODO scanning); string-literal token set; `create_element(node)` pure in the node type (PLAT-PSI-006); `create_file(view_provider)`; optional spacing rule between two tokens (*may*, *must*, *must not*, *line break*) used when the platform inserts nodes; optional whitespace reparse. Functions must be thread-safe and side-effect free. | Conformance suite (spec 02 section 4) passes for the language. |
| PLAT-LANG-031 | MUST | **Lexer**: *purpose* token stream for parsing and highlighting. *Contract*: `start(buffer, start, end, initial_state)`; `state()`; `token_type()` (none at end); `token_start()`, `token_end()`; `advance()`; `position()`/`restore(position)`; `buffer()`, `buffer_end()`. **Restartable**: for any token boundary with reported state S, `start(buffer, boundary, end, S)` produces the same tokens as continuing. Token ranges cover the whole input without gaps or overlaps; an unlexable character becomes a bad-character token; a lexer **never** panics or loops forever on any input. | Fuzz test: 100k random inputs; coverage, determinism and termination hold. |
| PLAT-LANG-032 | MUST | **Parser**: receives the tree builder positioned at the start (the file text, or the text of a lazy-parseable node) and the root element type; returns the finished tree; every marker is done or dropped; the parser makes progress on every iteration (never loops without consuming a token or emitting an error); recursion depth is bounded by the language (the platform imposes a depth limit, PLAT-PSI-046). | Fuzz: random token sequences do not hang or overflow. |
| PLAT-LANG-033 | MUST | **Element types**: each language declares its element and token types as interned values with the language attached; token sets are built once. A lazy-parseable type (chameleon) supplies a parse function; a reparseable type supplies `is_reparseable(node, new_text)` and `is_valid_reparse(old, new)`. | See PLAT-PSI-044, 050. |
| PLAT-LANG-034 | SHOULD | **View provider factory** (per file type or language): produces a custom provider for multi-tree/template files. | See PLAT-PSI-029. |
| PLAT-LANG-035 | MUST | **Language substitutor** (for a language, or for any language): pure function (file, project) to language or none; first non-null wins; the "any" substitutors run first; substituting to the "any language" value is an error. | See PLAT-VFS-077/078. |
| PLAT-LANG-036 | SHOULD | Languages that embed or are embedded (templates, markdown with code fences, JSON in strings) declare **injection** relations through injector contributions (below), not through special parser hacks. | Markdown fence with ```` ```java ```` produces a Java fragment. |

## 3. Editing-time language services

For each: **purpose / when called / contract**. All run under read access unless stated, on any thread unless stated, and must tolerate `Cancelled` by propagating it.

### 3.1 Syntax highlighter (lexer-based coloring)

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-LANG-040 | MUST | **SyntaxHighlighterFactory** (per language; may also be selected per file type): *purpose* coloring that needs only the lexer (keywords, comments, braces, literals). *Called*: when an editor is created, when the file type/language changes, when dumb mode changes (editor highlighters are rebuilt), and when the registration changes. *Contract*: `highlighter(project, file)` returns a highlighter (project and file may be none and may drive settings such as language level); requires read access; a default implementation highlights nothing. | Open a `.rs` file: tokens colored without any analysis. |
| PLAT-LANG-041 | MUST | **SyntaxHighlighter**: `lexer()` returns a **restartable** lexer (the editor re-lexes only the changed region by restarting at a token boundary with the saved state); `token_highlights(token_type)` returns an array of text-attribute keys; all keys are **merged** in order (later keys override earlier ones) to produce the final color/style. The same token type must always map to the same keys. | Type inside a block comment: only the affected lines re-lex (instrumented count). |
| PLAT-LANG-042 | MUST | Editor highlighting is **independent of the tree**: it must work (a) before the tree exists, (b) in dumb mode, (c) for files above the intelligence limit if the lexer can handle them (otherwise plain). | Open a 3 MB file: lexer highlighting still works within the frame budget, or the file is plain, never frozen. |

### 3.2 Annotator and external annotator (semantic highlighting and errors)

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-LANG-050 | MUST | **Annotator**: *purpose* add highlights, messages and quick fixes based on the tree and resolve. *Called*: by the editor's annotator pass for each element of the file (elements in the visible range first, then the rest), after each commit that changes the file, and again on dumb-mode transitions. *Contract*: `annotate(element, holder)`; **no state kept between calls** (state may be kept only within one call); not re-entered concurrently for one instance, but several instances may run in parallel; execution order among annotators is **unspecified**; **dumb-aware only if it says so**. | A stateful annotator is flagged in review; a non-aware annotator does not run while dumb. |
| PLAT-LANG-051 | MUST | **Annotation holder**: `new_annotation(severity, message)` (with message and tooltip) and `new_silent_annotation(severity)` (colors only) return a builder pre-bound to the current element; builder options: range (must lie **inside the current element**), text attributes key, tooltip, quick fixes (with optional range and problem group), highlight type (affects squiggle style), "after end of line", file-level, gutter icon, needs-update-on-typing, and `create()`. Nothing appears until `create()`. | An annotation with a range outside its element is clamped or rejected with a log (never shown at the wrong place). |
| PLAT-LANG-052 | MUST | **Severities** with numeric levels (a total order used for sorting, stripe marks and "next problem"): information 10, text attributes 11, server problem 100, weak warning 200 (an older alias "info" equals it), warning 300, error 400. Information and text-attribute annotations color text only (no gutter mark, not part of "next error" navigation). Custom levels may be registered between existing ones. | Sort a mixed list: errors first; information excluded from "next error". |
| PLAT-LANG-053 | MUST | Annotations produced for an element are **replaced wholesale** on the next run; stale ones never accumulate. Annotators must not annotate based on a different file than the element's. | Fix an error: its highlight disappears on the next pass. |
| PLAT-LANG-054 | SHOULD | **External annotator** (linters): three steps. (1) `collect_information(file, editor, has_errors)` in a read section: cheap, returns input data or none; skipped in dumb mode unless aware. (2) `annotate(info)` **outside** a read section, may be slow, must not touch PSI or indexes without taking its own sections; (3) `apply(file, result, holder)` in a read section adds annotations. Runs **after** regular annotators have finished. Each external annotator may be disabled by a filter and by an inspection toggle. | An external linter's results arrive seconds after the regular highlights. |
| PLAT-LANG-055 | MUST | An annotator's cost is bounded: the platform records time per annotator and per file; an annotator that exceeds a budget logs and may be auto-disabled for the session after repeated overruns (SHOULD). | Slow annotator shows in the daemon profile. |

### 3.3 Folding

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-LANG-060 | MUST | **FoldingBuilder**: *purpose* declare collapsible regions. *Called*: (1) on editor open with `quick = true` (no resolve, no heavy checks: used for auto-folding like import lists), (2) after each commit (full), (3) when the user changes folding settings. *Contract*: `build_fold_regions(root, document, quick)` returns descriptors; each has the element (or node), a text range, a **placeholder text** (shown when collapsed), an optional **group** (regions that expand/collapse together), and optional **dependencies** (elements whose change refreshes the region); `is_collapsed_by_default(descriptor)`; `keep_expanded_on_first_collapse_all(descriptor)`. Multiple builders for a language (and the any-language key) are combined; their descriptors are merged. | Open a Java file: import list is collapsed immediately (quick pass); method bodies fold after the full pass. |
| PLAT-LANG-061 | MUST | Region rules: non-empty ranges; regions are **properly nested or disjoint**; an improper overlap or empty range drops the region and logs; ranges are in file coordinates of the **committed** text. | A builder returning crossing ranges: one is dropped, no exception. |
| PLAT-LANG-062 | MUST | Folding state (collapsed or expanded) is **stored per file and restored** on reopen by region identity (the exact identity function is implementation-defined, for example kind, placeholder text and position); editing moves regions like range markers; regions that disappear are dropped; new regions use the default state. | Collapse a method, close and reopen: still collapsed. |
| PLAT-LANG-063 | SHOULD | "Expand all" and "Collapse all" are **two-step**: the first step skips regions the builder asks to keep (default-collapsed for expand, keep-expanded for collapse), the second affects all. | Press Collapse All twice: first keeps the class-level region, second collapses everything. |
| PLAT-LANG-064 | MUST | A folding builder is dumb-aware only if it says so; unaware builders produce no regions while dumb. | See PLAT-DUMB-053. |

### 3.4 Editing helpers

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-LANG-070 | MUST | **Commenter**: line comment prefix (and a list of accepted prefixes), block comment start/end, and the "commented" variants for escaping a nested end marker; a flag for languages whose block comment must occupy whole lines. Used by Comment with Line/Block Comment. Missing line prefix means the action falls back to block (and vice versa). | Toggle comment on three lines in a language with only block comments: one block. |
| PLAT-LANG-071 | MUST | **PairedBraceMatcher**: brace pairs (left token type, right token type, **structural** flag: the pair delimits code blocks used by "go to block start"), `is_paired_brace_before(left, context_token)` for auto-closing (true is always safe), and `code_construct_start(file, opening_brace_offset)` returning the start of the construct that owns the block (for showing it in the matching-brace tooltip). The language's highlighter lexer must produce the brace token types. | Type `{`: `}` is inserted when allowed; the matching brace highlights. |
| PLAT-LANG-072 | MUST | **Quote handler**: decides on typing or removal of a quote whether the quote is a closing quote (type over), an opening quote, whether it is inside a literal, and whether to insert a closing quote; works on a highlighting iterator (lexer-level), **not** the tree; multi-character quote variants and backspace handling are separate hooks. | Type `"`: pair inserted; type `"` again: caret moves over. |
| PLAT-LANG-073 | MUST | **Typed handler** (before/after character, auto-popup checks, before selection removal, "typing started" once per input regardless of carets): *Contract*: the file passed is **not guaranteed to be in sync** with the document (no commit is forced); handlers must be cheap and must not commit; to check PSI, **schedule** the check for later when the tree is current (for example, schedule the auto-popup with a condition evaluated on the up-to-date tree). Called once per caret except the "started" and the popup check (primary caret only). | A handler that forces a commit per keystroke is rejected in review; typing latency budget holds. |
| PLAT-LANG-074 | SHOULD | **Enter handler / smart enter / backspace handler** hooks follow the same no-forced-commit rule where documented. | n/a |
| PLAT-LANG-075 | MUST | **Extend/shrink selection handler**: default behavior follows tree ancestors; a handler may add intermediate ranges for a node (`can_select(element)`, `select(element, text, cursor_offset, editor)` returns ranges). Ranges returned must contain the cursor and be nested/ordered; they are merged into the default sequence. | In `f(a, b)` with `a` selected, extending selects "a, b" before the whole call (with such a handler). |
| PLAT-LANG-076 | MUST | **NamesValidator**: `is_keyword(name, project)` and `is_identifier(name, project)` for the language; used by Rename, Introduce, New File and similar to validate user-entered names; the default accepts nothing special (language-agnostic identifier rule). | Rename to `class` in Java: rejected as keyword. |
| PLAT-LANG-077 | SHOULD | **ImportOptimizer**: `supports(file)`; `process_file(file)` runs in a read section, does all calculation, and returns a **change action** that the platform runs in a write section (the heavy part must be in step 1); an action-mode switch lets slow auto-import run on the UI thread when needed. | Optimize Imports in a 5k-line file: UI stays responsive during calculation. |
| PLAT-LANG-078 | SHOULD | **Brace/indent/typing options** (code style settings, formatter, line wrapping) are specified by the formatting team (`FormattingModelBuilder`, `LanguageCodeStyleSettingsProvider`); registered per language with single lookup. | Pointer only. |

### 3.5 Navigation, search and structure

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-LANG-080 | MUST | **FindUsagesProvider**: `can_find_usages_for(element)`; `words_scanner()` (or none for the default simple scanner) used to build the word index (PLAT-IDX-081); `type(element)` (lower-case user-visible kind such as "class", "variable"); `descriptive_name(element)` (expanded name, e.g. the qualified name or a method signature); `node_text(element, use_full_name)` (text in the usage tree). All are used for UI texts, so they must be cheap. | Find Usages dialog title shows "class Foo". |
| PLAT-LANG-081 | MUST | **Reference contributions** (see spec 02 section 8.2): a language that wants contributed references must **opt in** by routing its element reference accessor through the registry; otherwise contributors have no effect. | A contributor on string literals has no effect in a language that did not opt in. |
| PLAT-LANG-082 | MUST | **ElementManipulator** (per element class): `handle_content_change(element, range, new_content)` rewrites the element's *value* text (e.g. a string literal's inner range) and returns the (possibly new) element; `range_in_element(element)` gives the *value range* (inside the quotes). Used by rename of references inside literals, injection editing, and "copy reference". **LiteralTextEscaper** decodes escapes from host text to value text and maps offsets back. | Rename a class referenced from a string `"com.acme.Foo"`: only the value text changes. |
| PLAT-LANG-083 | SHOULD | **ElementDescriptionProvider**: describes an element for a *location* (usage view, rename dialog, type info) as text; consulted in registration order; the first non-null wins; there is a default. | Rename dialog title uses "method foo". |
| PLAT-LANG-084 | MUST | **Structure view factory**: given a file, returns a builder of a tree model (nodes with presentation, children, sort/filter options, navigation target); *called* when the File Structure view/popup opens and on every commit while open; may be tree-only (no index); a language without a factory falls back to a generic one registered for any language. | Open File Structure on a Java class: methods and fields appear with icons. |
| PLAT-LANG-085 | MUST | **Go to Declaration handler** (extra targets beyond references): `targets(source_element, offset)` returns targets or none; consulted in addition to reference resolution; may supply custom action text. **Target element evaluator** decides which element at the caret is the *source* (e.g., prefers the reference over its enclosing statement). | Ctrl+click on a string key navigates to the property file entry through a handler. |
| PLAT-LANG-086 | MUST | **ChooseByNameContributor** (Go to Class / Symbol / File): `is_available_now(project)` (cheap, hides the tab if none), `names(project, include_non_project)`, `items_by_name(name, pattern, project, include_non_project)`; an extended form accepts a **scope**, a processor with early stop, and an id filter for speed. Names are unsorted; the platform filters, matches (camel-hump, fuzzy) and ranks. | Go to Class lists project classes first, libraries only when "include non-project items" is on. |
| PLAT-LANG-087 | SHOULD | **Line marker provider**: returns gutter markers for an element; the platform queries in **two passes** (visible range first, then the rest) and clears a provider's markers if both passes return nothing. *Contract*: create markers **on leaf elements** (for example the method name identifier) not on composite ones, otherwise a half-visible method flickers; provider is dumb-aware only if it says so. A "descriptor" variant gives the marker a name shown in Settings so users can hide it. | A marker on a whole method does not flicker when half-scrolled (negative test). |
| PLAT-LANG-088 | SHOULD | **Breadcrumbs provider**, **inlay hints provider**, **sticky lines**: pointer only (UI specs). | n/a |

### 3.6 Intelligence services of the JVM platform

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-LANG-090 | MUST | **PsiElementFinder** (`java.elementFinder`): class/package lookup contributions (see PLAT-IDX-099). Kotlin, Groovy and generated-code providers register finders. Each can be dumb-aware. | A Kotlin finder resolves a Kotlin class from Java source. |
| PLAT-LANG-091 | MUST | **PsiAugmentProvider**: add members, infer types, extension methods, modifier transforms, "initializer might be changed" (PLAT-PSI-151). Results for equal inputs must be equal objects (the platform may call repeatedly). | Annotation-processor-like members appear in completion and resolve. |
| PLAT-LANG-092 | MUST | **ResolveScopeProvider** (replace) and **ResolveScopeEnlarger** / **UseScopeEnlarger** (widen): see PLAT-PSI-131/132. | A plugin widens a build script's resolve scope. |
| PLAT-LANG-093 | SHOULD | **ImplicitUsageProvider / EntryPoint** (symbols used implicitly by frameworks: Spring beans, test methods, serialization hooks): suppress "unused" results. | A `@Bean` method is not reported unused. |
| PLAT-LANG-094 | SHOULD | **Class inheritor, overriding method, annotated-elements searches** are extensible *query executors* (spec for find usages); they must use the stub indexes of section 5.2 in spec 03 as their first filter. | Subclass search over a 100k-class project completes without parsing every file. |

## 4. Catalog of language extension points (who owns the details)

| Extension point (concept) | Lookup | Called when | Dumb-aware (required default; Archipel design) | Spec |
|---|---|---|---|---|
| Parser definition, lexer, parser | single | tree build | n/a | this (2.2) |
| File type, language file type, file type detector, language substitutor, file view provider factory | by file | file classification, tree build | n/a | 01, this |
| Syntax highlighter (factory) | single | editor create / rebuild | yes (lexer only) | this (3.1) |
| Annotator | all | annotator pass | no | this (3.2) |
| External annotator | all | after annotators | no | this (3.2) |
| Local inspection tool | all (by profile) | inspection pass | no | inspections spec (other team) |
| Intention action, quick fix | all | Alt+Enter, on highlights | no | intentions spec (other team) |
| Completion contributor | all, ordered | completion | no | completion spec (other team) |
| Folding builder | all (composed) | open/commit | no | this (3.3) |
| Commenter, brace matcher, quote handler, typed/enter/backspace handler, selection handler | single or all as noted | editing | yes where purely lexical | this (3.4) |
| Names validator | single | rename/new | yes | this (3.4) |
| Import optimizer | single | Optimize Imports | no | this (3.4) |
| Find usages provider, words scanner | single/all | find usages, word index | scanner: yes | this (3.5), 03 |
| Reference contributor / provider | per pattern | reference computation | no | 02 (8.2) |
| Element manipulator, literal escaper | per element class | rename/injection | yes | this (3.5) |
| Structure view factory | single | structure view | no | this (3.5) |
| Go to Declaration handler, target element evaluator | all | navigation | no | this (3.5) |
| Choose-by-name contributor | all | Go to ... | no | this (3.5) |
| Line marker provider | all | gutter pass | no | this (3.5) |
| Documentation provider | single | quick doc | no | docs spec (other team) |
| Parameter info handler | single | parameter hints | no | editing spec (other team) |
| Formatting model builder, code style settings provider | single | reformat | no | formatting spec (other team) |
| Refactoring support provider | single | rename/safe delete/etc. | no | refactoring spec (other team) |
| Language injector, multi-host injector | all | injection pass | no | 02 (10) |
| Stub element types, stub index extensions | per language | indexing | no | 03 |
| File-based index extensions | global | indexing | no | 03 |
| Element finder, augment provider, scope providers/enlargers | global | resolve | no | this (3.6), 02, 07 |
| Spell-check strategy, suppressors (`@SuppressWarnings` analogs) | single/all | inspections | no | inspections spec |

## 5. Platform rules shared by all language services

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-LANG-100 | MUST | **Dumb-awareness is declared per service** (PLAT-DUMB-030) with the defaults of section 4; the registry reports services that are not aware while dumb mode is active. | See PLAT-DUMB-031. |
| PLAT-LANG-101 | MUST | **No service may keep per-file or per-project mutable state** in its instance; use cached values on files or services keyed by project (PLAT-CACHE-030). | Review lint. |
| PLAT-LANG-102 | MUST | **Cancellation**: every service call can be cancelled; services propagate `Cancelled` and never swallow it (PLAT-CONC-021). | See spec 06. |
| PLAT-LANG-103 | MUST | **Errors in services are isolated**: a panic or error in one language service is caught at the call site by the infrastructure, logged with the service and language, and does not break other services or the editor; repeated failures may disable the service for the session. | A panicking annotator does not stop other annotators; the editor stays usable. |
| PLAT-LANG-104 | MUST | **Registration conformance test**: each language ships a conformance test that instantiates every registered service for its language and exercises it on a small corpus (parse, highlight, fold, annotate) without panics and within budgets. | CI job per language. |
| PLAT-LANG-105 | SHOULD | A language declares its **completeness tier** per feature family (code-intelligence spec D6) so inspections that need exact resolve can mute themselves for lower tiers. | A tier-G1 language does not report unresolved references. |
| PLAT-LANG-106 | SHOULD | Services receive the **file's language** (not only the registered language) when called through a dialect/base fallback (PLAT-LANG-014). | A dialect-specific token is not mis-highlighted by base rules. |

## 6. Minimum service set for a first-release language

| Language class | Required (MUST) | Recommended (SHOULD) |
|---|---|---|
| **JVM (Java, Kotlin)** | language, file type, parser definition (or a bridge to the existing parser), lexer, syntax highlighter, stub builder + stub indexes (spec 03), reference providers (resolve), annotator (unresolved references and syntax errors), folding builder, commenter, brace matcher, quote handler, names validator, find usages provider + words scanner, structure view factory, class finder + short-name cache contributions, resolve scope participation | line markers, import optimizer, go-to-declaration handler, selection handler, injectors, augment provider |
| **Spring configuration (`application.properties`/`yml`, XML)** | language/file type for properties and YAML, parser + highlighter, reference contributors for property keys and bean names, structure view, folding for properties, find usages for keys | external annotator for schema checks, line markers for bean usages |
| **Other data languages (JSON, XML, TOML, SQL)** | file type, highlighter, brace matcher, commenter, folding, structure view | reference contributors, injection |

## 7. Notes for implementers (non-binding)

* Archipel's existing tree-sitter based generic features (folding via queries, tags-based structure, bracket matching) can be offered as **default implementations** of the corresponding extension points for a language that only provides a grammar. The lookup rules of PLAT-LANG-010 make a hand-written service override the default per feature without changing callers (decision D9 in the code-intelligence spec).
* The registry should make "which services does language X have, which are dumb-aware, which are default" a data query (for a developer tools window and for the conformance tests of PLAT-LANG-104).
* The "typed handler must not force a commit" rule (PLAT-LANG-073) interacts with Archipel's synchronous incremental reparse: if the tree is always current when handlers run, handlers *may* use it, but the contract in the spec is the conservative one and must not be relied on by service authors, because later Archipel versions might move parsing off the UI thread.

## 8. Open questions

1. Whether Archipel exposes extension points to **third-party plugins** in the first release or only to built-in language packs. The contracts above (statelessness, isolation, cancellation) matter either way; isolation (PLAT-LANG-103) is MUST in both cases.
2. The studied tree is introducing a *symbol* API next to PSI references (declaration/reference providers keyed by symbol); services in section 3.5 would then have symbol-based siblings. Not specified here.
3. Combination rule for several Go to Declaration handlers and reference targets (concatenate, first wins): defer to the navigation spec.

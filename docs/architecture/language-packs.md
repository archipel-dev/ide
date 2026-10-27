# Language packs

Status: first slice of the language extension points (spec
[`docs/specs/platform/08-language-extension-points.md`](../specs/platform/08-language-extension-points.md),
IDs `PLAT-LANG-*`). Java is the reference pack; Rust, Kotlin, TOML, Markdown and Bazel moved
with it. Code: `crates/archipel-ide-langpacks` (the packs), `archipel-ide-analysis::langext`
(the language-bound extension points), `archipel-ide-lsp` and `archipel-ide-toolchains` (the
points of those layers), `crates/archipel-ide-core/src/language_packs.rs` (the host side).

`AGENTS.md`: "Language integrations belong behind plugin/service traits, never inside the
platform core." A language is a **pack**: a bundle of extensions registered at application start
on the extension points of the platform kernel (`ExtensionPointName`, `ExtensionRegistry`,
`LoadingOrder`, `Disposer`; see [`platform-kernel.md`](platform-kernel.md)). Core, daemon and UI
read the extension points and never name a language. The next language (Python, Go, ...) adds a
pack and nothing else.

```
LanguagePack::register(cx)            archipel-ide-langpacks/src/packs/<language>.rs
   |  cx.tree_sitter_language("java")      Language + FileType + ParserDefinition
   |  cx.language_extension(HIGHLIGHTING_PASS, Registration::new("java-resolve", JAVA), ..)
   |  cx.language_server(jdtls)            LANGUAGE_SERVER_PROVIDER
   |  cx.toolchain_requirement(..)         TOOLCHAIN_REQUIREMENT_PROVIDER
   |  cx.import_fix_provider(..) / cx.activity(..)
   v
ExtensionRegistry (application)  <-- every extension is a child of the pack's disposer node
   |  change listeners (language_packs.rs: PacksDirty)
   v
IdeState::sync_language_packs()  once per poll, only when something changed
   |-- analysis registry   registry_from_extensions(base services + language extensions)
   |-- LspManager          set_contributed_providers(LANGUAGE_SERVER_PROVIDER, loading order)
   `-- toolchain bars      requirements_from(TOOLCHAIN_REQUIREMENT_PROVIDER)
```

## How to add a language pack

1. **Grammar and data.** A tree-sitter language is an entry of the grammar table
   (`crates/archipel-ide-language/src/registry.rs`: grammar, queries, token sets, extensions).
   That table stays the data source (see "Not done yet"). Without a grammar, declare the
   `Language` and `FileType` by hand (`PackContext::language`, `PackContext::file_type`).
2. **The pack.** `crates/archipel-ide-langpacks/src/packs/<language>.rs`:

   ```rust
   pub struct PythonPack;
   impl LanguagePack for PythonPack {
       fn id(&self) -> &'static str { "python" }
       fn display_name(&self) -> &'static str { "Python" }
       fn register(&self, cx: &mut PackContext<'_>) {
           cx.tree_sitter_language("python", |language| language);      // PLAT-LANG-001/020/030
           cx.language_extension(                                       // PLAT-LANG-050
               HIGHLIGHTING_PASS,
               Registration::new("python-resolve", LanguageId("python")).dumb_aware(true),
               Arc::new(PythonResolvePass::new()),
           );
           if let Some(server) = archipel_ide_lsp::builtin_provider("pyright") {
               cx.language_server(server);                              // language -> server
           }
           cx.toolchain_requirement("python", python_requirement, None);
       }
   }
   ```

3. **Install it** in `packs::builtin_packs()` (`packs/mod.rs`). Order matters only where
   packs share a point (toolchain requirements are listed in install order, with
   `listed_after` for "right after Java").
4. **State and per-frame work.** A pack with indexes or settings implements `LanguageActivity`
   (`poll` once per frame with settings, project model and open buffers; `paths_changed`;
   `status` for the control socket) and registers it with `cx.activity(..)`. Keep the state in
   the pack's own service object, not in `IdeState` (see `packs/java.rs`).
5. **Import fixes with a chooser** implement `ImportFixProvider`
   (`cx.import_fix_provider(..)`) and list their command prefix in
   `LanguagePack::import_command_prefixes`; the host registers `<prefix>.chooser`,
   `<prefix>.chooseImport` and `<prefix>.cancelChooser`.
6. **Test.** Add the language to the corpus of
   `crates/archipel-ide-langpacks/tests/packs.rs` (PLAT-LANG-104: instantiate what is registered
   and exercise it). `tests/packs.rs` also holds the install/uninstall and loading-order tests
   to copy from.

Rules (spec 08): extensions are shared application-level instances, stateless or thread safe
(PLAT-LANG-013, PLAT-LANG-101); a dialect passes through its base language's extensions
(PLAT-LANG-014); cancellation is propagated (PLAT-LANG-102).

## Install, remove, order

* `LanguagePacks` (application service) installs a pack under a new node of the disposer
  tree. A failed install (duplicate id, unknown grammar) disposes the node: nothing stays
  registered. `uninstall("java")` disposes the node, so **every contribution of the pack
  leaves** (PLAT-LANG-005); the host notices through the extension change listeners and
  rebuilds its caches on the next poll.
* `LoadingOrder` is the order within a point: `Registration::order(Order::First | Last |
  Before(id) | After(id))` on language-bound points, `LoadingOrder` on the others.
  `registry_from_extensions` hands the registrations to the `ServiceRegistry`, which resolves
  specificity (language, then base language, then "any") and the constraints (PLAT-LANG-010,
  -011, -014). Same id in the same language on one point is an error, not an override.
* Dynamic loading is not needed yet: packs are registered statically at start
  (`install_language_packs` in `language_packs.rs`, called by `Platform::default`).
* `LanguagePacks::services_of(language)` answers "which services does language X have, which
  are dumb aware" as data (spec 08 section 6).

## Extension points

Names are `dev.archipel.language.<name>`. "Language bound" points take a `Registration`
(`id`, language or "any", `Order`, `dumb_aware`) and are looked up per language with base
fallback; the others are plain lists.

| Extension point | Type | Spec | Status |
|---|---|---|---|
| `LANGUAGE` | `Language` (id, name, base, case sensitivity, tier, contributed references) | PLAT-LANG-001..003, -005, -105 | done; `is_kind_of` for dialects. Marker categories (PLAT-LANG-004), MIME types, language substitutors (-035): not yet |
| `FILE_TYPE` | `FileType` (extensions, exact names) | PLAT-LANG-020, -021 | done: `file_type_for_path`, `language_for_path`. Hashbang, binary/read-only flags, charsets, lazy factories (-022): not yet |
| `PARSER_DEFINITION` (language bound) | `dyn ParserDefinition` (hands out the tree-sitter `LanguageDef`) | PLAT-LANG-030..033 | done as data; the grammar table is still static |
| `HIGHLIGHTING_PASS` (language bound) | `dyn Pass` | PLAT-LANG-050, -055 | done: Java `java-resolve` |
| `ANNOTATOR`, `EXTERNAL_ANNOTATOR` | `dyn Annotator`, `dyn DynExternalAnnotator` | PLAT-LANG-050..054 | point declared, daemon reads it through the registry |
| `LOCAL_INSPECTION_TOOL`, `GLOBAL_INSPECTION_TOOL`, `INSPECTION_SUPPRESSOR`, `IMPLICIT_USAGE_PROVIDER` | `LocalInspection`, `GlobalInspection`, `InspectionSuppressor`, `EntryPoint` | PLAT-LANG-050, -093 | declared; Java and Rust suppressors moved into their packs; Tier-A inspections still registered by the daemon |
| `INTENTION_ACTION` (+ quick fixes returned by passes) | `dyn IntentionAction` | PLAT-LANG-050 | declared |
| `IMPORT_FIX_PROVIDER` | `dyn ImportFixProvider` (chooser state per provider) | PLAT-LANG-050 | done: Java; the chooser commands and the UI popup are generic |
| `COMPLETION_CONTRIBUTOR`, `COMPLETION_WEIGHER` | `CompletionContributor`, `CompletionWeigher` | spec 08 section 4 | declared |
| `REFERENCE_CONTRIBUTOR` (language bound) | `dyn ReferenceProvider` (+ `priority()`) | PLAT-LANG-081, PLAT-PSI-110..113 | declared with the merge rule: `merge_references` (highest priority group kept, lower groups only where ranges do not intersect) and the opt-in `Language::contributed_references` in `contributed_references` |
| `RESOLVER`, `USAGE_SEARCHER`, `READ_WRITE_ACCESS_DETECTOR` | `Resolver`, `UsageSearcher`, `ReadWriteAccessDetector` | PLAT-LANG-080, -094 | declared |
| `FOLDING_BUILDER` | `dyn FoldingBuilder` | PLAT-LANG-060..064 | declared (generic tree-sitter folding registered for "any" by `default_registry`) |
| `STRUCTURE_VIEW_BUILDER` | `dyn StructureProvider` | PLAT-LANG-084 | declared |
| `COMMENTER`, `BRACE_MATCHER`, `QUOTE_HANDLER`, `TYPED_HANDLER`, `SMART_ENTER_PROCESSOR`, `EXTEND_SELECTION_HANDLER`, `INDENTER`, `MOVE_STATEMENT_HANDLER` | the traits of `analysis::editing` | PLAT-LANG-070..075, -078 | declared |
| `STICKY_SCOPES_PROVIDER`, `BREADCRUMBS_PROVIDER`, `DOCUMENTATION_PROVIDER`, `PARAMETER_INFO_HANDLER`, `FORMATTING_SERVICE`, `LIVE_TEMPLATE_CONTEXT`, `REFACTORING_SUPPORT`, `REFACTORING_LISTENER`, `INTERFACE_SUMMARIZER` | the traits of `analysis` | PLAT-LANG-012, -078, -088 | declared |
| `LANGUAGE_SERVER_PROVIDER` (`archipel-ide-lsp`) | `dyn LanguageServerProvider` (+ `toolchain_component()`) | PLAT-LANG-012 | done: `ServerRegistry::from_extensions` / `LspManager::set_contributed_providers` |
| `TOOLCHAIN_REQUIREMENT_PROVIDER` (`archipel-ide-toolchains`) | `dyn ToolchainRequirementProvider` | PLAT-LANG-012 | done: Java, Kotlin, Rust, Bazel; `requirements_from` with `listed_after` |
| `LANGUAGE_ACTIVITY` (`archipel-ide-langpacks`) | `dyn LanguageActivity` (settings, per-frame poll, watcher, status) | PLAT-LANG-012, -013 | done: Java |

"Declared" means: the point exists, a pack can register on it, `registry_from_extensions`
copies it into the query registry, and the daemon/UI already consume that registry
(`registry().all::<dyn T>(language)`). The macro that declares a point also lists it in
`ALL_POINTS`, so a declared point cannot be missed by the bridge. Only the ones marked "done"
have a built-in contributor today.

Priorities of spec 08 taken now (MUST): 001, 002, 003, 005 (removal), 010, 011, 012, 013, 014,
020, 021, 030 (as data), 050, 081 (merge rule), 100, 101, 104 (conformance corpus). Later: 004,
022, 031/032 (lexer and parser contracts beyond tree-sitter), 035, 036, 054 contract details,
055 (per-annotator budgets), 062..064 (folding state), 072..077, 080, 082..088, 090..094.

## The packs

| Pack | Contributes |
|---|---|
| `java` | language, file type, parser; `java-resolve` pass (unresolved references, import fixes, unused imports); `@SuppressWarnings` suppressor; `ImportFixProvider` + chooser commands `java.chooser` / `java.chooseImport` / `java.cancelChooser`; `LanguageActivity` (indexes follow the project model and buffers, the three `java.*` settings, `native_wins` for the daemon); jdtls; JDK + jdtls + java-debug requirement |
| `rust` | language, file type, parser; `#[allow]` suppressor; rust-analyzer; rust-analyzer requirement |
| `kotlin` | language, file type, parser; kotlin-lsp; kotlin-lsp requirement (listed after Java) |
| `toml` | language, file type, parser; taplo |
| `markdown` | `markdown` and the dependent `markdown_inline` language, file type, parser |
| `bazel` | Starlark language, file type, parser; starlark server; bazelisk requirement |
| `catalog` | the language servers of languages that have no pack yet (pyright, gopls, typescript, clangd, lemminx, yaml, json) so nothing is lost until those get packs |

## Host side (what replaced the Java wiring)

* `IdeState::sync_language_packs` (start of `poll_analysis` and `poll_daemon`): rebuilds the
  analysis registry (`AnalysisServices::apply_extensions`: the platform's services such as the
  daemon are cloned from `base_registry`, so their state survives), the language server
  providers and the toolchain requirements, when an extension point changed.
* `IdeState::poll_language_activities` (was `poll_java`): adds pack settings to Settings |
  Editor | Inspections, polls every `LanguageActivity` and applies `native_wins` to the daemon.
* Import chooser: `IdeState::import_chooser`, `choose_import`, `open_import_chooser`; the UI
  popup and the control socket (`/chooser`, `/languages/<pack>/state`) use them.
* The control snapshot reports `languages.<pack id>.state` instead of `java.state` (scene 36).

Test: `crates/archipel-ide-core/tests/language_packs.rs` (removing the Java pack removes the
pass, the activity, jdtls and the chooser; commands refuse politely; the rest keeps working) and
`crates/archipel-ide-langpacks/tests/packs.rs`.

## Not done yet (next PRs)

* **The grammar table** (`archipel_ide_language::LANGUAGES`, `detect`, `by_id`) is still static
  data used by about 50 call sites for highlighting, folding, injection and file detection;
  packs register `Language`/`FileType`/`ParserDefinition` descriptors that mirror it, so
  removing a pack does not stop syntax highlighting of its files. Next: make the table a
  consumer of `PARSER_DEFINITION` and add packs for the remaining 15 grammars (XML, JSON, YAML,
  Python, JavaScript/TypeScript/TSX, Go, C/C++, Bash, HTML, CSS, properties, Groovy).
* **Server catalog.** `archipel_ide_lsp::builtin_specs()` keeps the data of every well-known
  server (`jdtls` launch logic in `JdtlsProvider`, ids, install hints, `server_component`);
  packs pick providers by id. Next: move each spec into its pack, and `JdtlsProvider` into the
  Java pack.
* **Toolchain resolution.** `ToolchainServerResolver` in `core/src/toolchain_lsp.rs` still has
  the jdtls Equinox launcher and the Kotlin server argument rules. They belong to the packs'
  `toolchain_component`-style hooks.
* **Language names left in core, daemon and UI** (none of it is wiring of a language's
  intelligence): the Java debug adapter (`java_debug.rs`, `debug_commands.rs`), file templates
  and new-class dialogs (`new_class.rs`, `JavaKind`, `KotlinKind`), Spring generation, test
  navigation (`test_nav.rs`), the project view's Java source roots, Gradle/Maven build wiring,
  Android, the completion heuristics for `Type name` vs `name: Type`, the
  `java.apply.workspaceEdit` jdtls command (`code_actions.rs`), the Tier-A inspection tables
  (`daemon/src/inspection/kinds.rs`) and the Java/Rust suppressor implementations
  (`suppress.rs`, registered by their packs), and fixture-heavy tests.
* **Contract details of spec 08:** per-annotator time budgets and auto-disable (-055), error
  isolation around every service call (-103), cancellation audit (-102), dumb-awareness report
  for services (-100: `Registration::dumb_aware` exists and `services_of` reports it, no
  runtime report yet), language substitutors (-035), injection relations (-036), element
  manipulators (-082), name validators (-076), go-to-declaration handlers (-085), choose-by-name
  contributors (-086), line markers (-087).
* **Dynamic loading** of third-party packs (spec 08 open question 1): the contracts (stateless
  extensions, isolation, cancellation) are in place, the loader is not.
* A pack that is installed after start works (the host rebuilds on change), but the chooser
  commands of a pack are registered only for the built-in packs because the command registry is
  built once.

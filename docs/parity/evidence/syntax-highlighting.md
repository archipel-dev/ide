# Syntax highlighting and color schemes (scene 04)

Captured 2026-10-10 on Linux under Xvfb (`:99`, 1600x1000x24, openbox) with
`ARCHIPEL_RENDERER=glow ARCHIPEL_NO_UPDATE_CHECK=1`, debug build of
`claude/tree-sitter-highlighting`. Screenshots: `import -window root`, then
`convert -strip -colors 256`.

Reference: `docs/parity/evidence/reference/intellij-community/java-gradle/04-editor-project-tree.png`
(IntelliJ IDEA Community 2025.2.6.3) and
`docs/parity/evidence/reference/android-studio/04-editor-project-tree.png`
(Android Studio 2026.2.1), from PR #40. Before this change every one of these files rendered as
single-colour text (`docs/parity/evidence/archipel-ide/*/04-editor-project-tree.png`, PR #40).

## How highlighting works

- `archipel-ide-language::registry` lists every language as data: grammar crate, file extensions,
  file names, shebang interpreters and the queries the crate ships (`HIGHLIGHTS_QUERY`,
  `INJECTIONS_QUERY`, `LOCALS_QUERY`, `TAGS_QUERY`).
- `archipel-ide-language::highlight` runs the highlight query over the syntax tree with
  `tree_sitter::QueryCursor` (byte-range bounded) and resolves overlaps with the
  `tree-sitter-highlight` rules: the last matching pattern wins for a node, and inner captures
  override outer ones. The output is a `HighlightLayer`. LSP semantic tokens can later be stacked
  on top with `HighlightLayer::overlay`.
- Capture names (`keyword`, `function.method`, `type.builtin`, …) map to IntelliJ text-attribute
  keys (`DEFAULT_KEYWORD`, `DEFAULT_FUNCTION_CALL`, …). The lookup is hierarchical, and a language
  can override single captures as data (XML/HTML tag keys). Generic structural refinements turn a
  declared function name into `DEFAULT_FUNCTION_DECLARATION`, `/** */` and `///` comments into
  `DEFAULT_DOC_COMMENT`, numeric constants into `DEFAULT_NUMBER`, capitalised `type.builtin`
  (`String`) into a class reference, and quotes into part of the string.
- `archipel-ide-language::scheme` loads the bundled IntelliJ schemes from
  `docs/spec/data/color-schemes.json`. It follows the scheme lineage
  (Default → Darcula → Islands Dark), resolves `inherit` references and falls back to `DEFAULT_*`
  keys. The editor resolves every key through Islands Dark.

## Incremental parsing

- `archipel-ide-text::TextDocument` journals every rope mutation (edits, coalesced typing, undo and
  redo) as a `ByteEdit` (bytes plus row/byte-column points). New documents start with
  `EditJournal::Reset`, and a journal with more than 4096 undrained edits also resets.
- `Document::refresh_analysis` drains the journal. `LanguageAnalysis::apply_edits` applies each
  edit to the tree (`Tree::edit`) and to the highlight layer, then reparses with the old tree.
  Only these ranges are re-highlighted: the edited lines, `Tree::changed_ranges`, the lines of
  `ERROR` nodes (`changed_ranges` misses children relabelled inside an unchanged `ERROR`), and
  spans the edit cut. Each range grows until it covers whole captures and whole old spans, so no
  clipped remainder keeps a stale key (for example a comment that lost its `/*`).
- `tests/incremental.rs` (proptest) applies random edits, undo and redo to the Java, Kotlin, Rust,
  Bazel and XML samples through the real `TextDocument`. After every step the incremental
  highlights must equal a full highlight of the same tree, and the tree and highlights must equal a
  fresh analysis whenever the text parses without errors. Error recovery may legitimately differ
  between incremental and fresh parses of broken text. Locally it passed 20,000 cases.
- `syntax-highlighting/04-java-typing-incremental.png`: line 7 was typed with `xdotool type`
  (`/* new comment */ int answer = 42; String s = "typed";`) into the running app (debug build).
  Every keystroke went through the incremental path, and the comment, keyword, number and string
  are coloured while typing.
- Measured (release, `Big.java`, 10,004 lines, 293 KB): full analysis 98.6 ms; one typed
  character 5–7 ms, of which about 4.3 ms is the Tree-sitter incremental reparse and the rest
  highlight splicing. One keystroke re-highlights about 80 bytes.

## Injections

`injections.scm` from the grammar crates drives injected highlighting
(`archipel-ide-language::injection`). It follows the `tree-sitter-highlight` conventions:
`@injection.content`, `@injection.language` or the `injection.language` property,
`injection.combined`, `injection.include-children`, and legacy captures named after the language.
Content is parsed with `Parser::set_included_ranges` (named children are excluded unless
`include-children` is set), highlighted recursively up to depth 4, and overlaid on the host
highlights; gaps keep the host capture. Fence info strings resolve through `registry::by_id`
aliases (`kt`, `sh`, `c++`, …). Languages without a grammar (regex, jsdoc, latex) are skipped.
During incremental re-highlighting, a range grows over every injection that overlaps or touches it,
because an injection that gains or loses text at an edge gets a different injected parse. The
property test includes Markdown and HTML samples.

- `syntax-highlighting/injections-markdown-readme.png`: heading, inline code spans, a link and
  autolink (Markdown inline grammar injected into `inline` nodes), a fenced `kotlin` block
  highlighted by the Kotlin grammar and a fenced `xml` block by the XML grammar. Still differs
  from IntelliJ: `**bold**` and `*italic*` are not rendered bold or italic, because the scheme
  data has no Markdown emphasis keys.
- `syntax-highlighting/groovy-build-gradle.png`: Groovy `build.gradle` (`dekobon-tree-sitter-groovy`)
  with strings, numbers, comments, `def` and GString interpolation.

## Extend / Shrink Selection

`archipel-ide-language::selection::expand` starts at the smallest node around the selection and walks
up the tree. At each node it returns the first candidate that strictly contains the current
selection, trying in order: the word under the caret inside a comment or string, the contents
between the node's first and last anonymous children (brackets, quotes, tags; trimmed), and then
the node itself. `editor.extendSelection` applies this to every caret. `editor.shrinkSelection`
walks back the recorded history for as long as each remembered selection still lies inside the
current one. Keys follow `docs/spec/keymaps.md`: Ctrl+W / Ctrl+Shift+W on Windows/Linux, ⌥↑ / ⌥↓
on macOS.

- `syntax-highlighting/extend-selection-ctrl-w.png`: the caret was in `greeting` on line 11, then
  Ctrl+W was pressed three times (`greeting` → `app.greeting(name)` → `(app.greeting(name))`).

## Code folding

`archipel-ide-language::folding` derives regions from the tree. A grammar's own `folds.scm` is used
when one ships (Starlark). Otherwise the regions are: nodes delimited by `{}`/`[]`/`()`/`<>` or
start and end tags on different lines, block comments, and runs of import declarations, with at
most one region per start line. Queries are bounded by byte range: the editor asks only for the
viewport (~60 µs on the 10k-line file), and Collapse All asks for the whole file. Collapsed regions
live in `Document::folds` as byte anchors that follow the edit journal. An edit inside a region
expands it, and so does a caret that enters hidden text (Go to Line, Find, arrows), as in IntelliJ.
The editor maps visual rows to document lines (`archipel-ide-ui::folding::FoldView`), renders a
collapsed region as prefix + placeholder (`FOLDED_TEXT_ATTRIBUTES`) + the rest of its last line,
and moves Up/Down by visual rows.

- `syntax-highlighting/04-kotlin-imports-folded.png`: `MainActivity.kt` as it opens. The imports
  are collapsed to `import ...` with a `›` gutter marker, and line numbers jump 3 → 6, as in
  Android Studio scene 04.
- `syntax-highlighting/folding-gutter-collapse.png`: after clicking the class's gutter marker the
  body shows as `{...}`. The next visible line is 16.

## Structure, File Structure popup, breadcrumbs

`archipel-ide-language::structure` runs each grammar's tags query (`@definition.<kind>` with `@name`).
Java uses upstream plus `queries/java/tags.scm` (constructors, fields, enums, records); Kotlin uses
the crate's own `tags.scm`, vendored with attribution because the crate does not export it.
Definitions are nested by byte containment. Callables get an IntelliJ-style signature built from
the tree's parameter `type` fields and the return type (`greeting(String): String`,
`onCreate(Bundle?)`). The UI caches the structure per document revision.

- `syntax-highlighting/22-file-structure-popup.png` vs IntelliJ `java-gradle/22-structure.png`:
  Ctrl+F12 with the caret in `main`. Like the reference it has the centred `App.java` title,
  `App` → `greeting(String): String`, `main(String[]): void`, and `main` preselected. Still
  differs: there are no "Inherited members / Anonymous classes / Lambdas" checkboxes, no
  visibility badges and no expand chevrons; the filter is a visible field rather than IntelliJ's
  speed search.
- `syntax-highlighting/22-structure-tool-window.png`: the Structure tool window (Alt+7, left
  stripe) highlights the declaration at the caret. The status-bar navigation path reads
  `java-gradle › src › main › java › com › example › App › main`, as in the reference.

## Matching brace, markup and data structure

- The highlight for the matching brace now comes from the tree
  (`archipel-ide-language::brackets`). Only real bracket tokens match, and the partner is the
  complementary token among the same node's children. A `(` inside a string or comment never
  matches; files without a grammar keep the text matcher.
- Structure for languages whose grammars ship no tags query uses Archipel IDE-owned
  `queries/<lang>/tags.scm` data: XML/HTML elements, JSON/YAML/TOML keys and tables, and Markdown
  headings (nested through tree-sitter-md sections). Qualified tag names are shortened in the
  navigation bar (`ConstraintLayout`).
- `syntax-highlighting/structure-android-layout-xml.png`: the Structure window for the Android
  layout (`ConstraintLayout` › `TextView`), with the caret's element highlighted and the
  breadcrumbs shortened.

## Languages

Rust, Java, Kotlin (`tree-sitter-kotlin-sg`), Groovy (`*.gradle`, `*.groovy`, `Jenkinsfile`), XML, JSON, TOML, YAML, Markdown (block grammar, with the inline grammar injected),
Starlark (`BUILD`, `BUILD.bazel`, `WORKSPACE`, `MODULE.bazel`, `*.bzl`), Python, JavaScript/JSX,
TypeScript, TSX, Go, C, C++, Bash (including `gradlew` and shebangs), HTML, CSS and Properties.

The only query that Archipel IDE owns is `crates/archipel-ide-language/queries/kotlin/highlights.scm`. It
is appended to the upstream Kotlin query and fills two upstream gaps: there is no `package`
keyword capture, and function names after modifiers are missed.

## Scenes

| File | Reference scene | Matches | Still differs |
|---|---|---|---|
| `syntax-highlighting/04-java-app.png` | IntelliJ java-gradle 04 | keywords (`package`, `public`, `class`, `void`, `new`, `for`, `return`) orange `#CF8E6D`; strings green `#6AAB73`; method declarations `greeting`/`main` blue `#56A8F5`; class names, type uses and calls in the identifier colour, as in IntelliJ | `System.out` (static field, purple italic) and `List.of` (static call, italic) need semantic information (LSP layer); no inlay hints, gutter icons or folding yet |
| `syntax-highlighting/04-kotlin-main-activity.png` | Android Studio 04 (same file content) | `package`/`import`/`class`/`override`/`fun`/`private`/`super` orange; `onCreate`/`greet` declarations blue; strings green; `String` and other types plain | `R.layout`/`R.id` qualifiers show purple (all navigation suffixes are captured as properties) where Android Studio draws only the resource field in purple italic with an underline; `$name` template entries are not coloured separately; imports are not folded |
| `syntax-highlighting/a04-xml-layout.png` | Android Studio a04 (code view) | tag names and `<`/`/>` delimiters `#D5B778` (`XML_TAG_NAME`, `XML_TAG`); attribute values green including quotes; comments grey; `&amp;` entity | `xmlns:`/`android:` prefixes are not coloured separately; the prolog version is coloured as a number |
| `syntax-highlighting/04b-build-bazel.png` | Archipel IDE PR #40 `bazel/04b` (was plain text) | Starlark detected from `BUILD.bazel`; strings, comments, punctuation | IntelliJ Bazel plugin keyword-argument colours are not reproduced |
| `syntax-highlighting/04-rust-main.png` | IntelliJ scene 04 palette (Rust file; Xvfb, glow, `archipel-ide-control` screenshot) | doc comment green italic; `use`/`fn`/`let`/`mut`/`for`/`in`/`struct`/`impl` orange `#CF8E6D`; function declarations `sum`/`main` blue; macro call `println!` blue; strings green `#6AAB73`; numbers `1.5_f64`/`2.0`/`0` blue; attribute `#[derive(...)]` yellow; one-line body folded as `{ self.x + self.y }` | no IntelliJ Rust plugin capture to compare against (the reference IDE has no Rust scene); lifetimes and generics parameters use the plain identifier colour |

## Tests

- `registry::tests`: detection by extension, exact file name (Bazel files, `Cargo.lock`, `gradlew`),
  shebang (`env -S`), aliases, uniqueness.
- `highlight::tests`: every bundled query compiles and maps captures; overlap resolution; layer
  overlay and range clipping.
- `scheme::tests`: Islands Dark and Light values from `docs/spec/themes.md` (keyword, string,
  number, comments, doc comment italic, constants, `XML_TAG_NAME`, `inherit` of
  `DEFAULT_FUNCTION_CALL`, errors wave underline `#FA6675`, editor colors).
- `tests::java_tokens_resolve_to_islands_dark_colors`: Java keywords, strings, comments, doc
  comments, declarations vs. calls and syntax errors resolve to the spec colours (issue #9
  acceptance).
- `tests::scene_04_languages_highlight`: one or more tokens per language.

# Platform spec 02: PSI model, references, resolve contract, element lifecycle

Status: specification (clean-room). ID prefix `PLAT-PSI`. Priorities: `MUST` / `SHOULD` / `LATER` as in `README.md`.

Supersedes and deepens sections 1.2 and 1.5 (platform parts) of `docs/analysis/code-intelligence-spec.md`.

"PSI" is IntelliJ's name for the typed, navigable, per-file program structure model that every code-intelligence
feature talks to. This document specifies what the model *promises*: shape, text coverage, identity, validity,
change notification, and the reference/resolve contract. It does not prescribe a Rust type layout; Archipel
may build it over tree-sitter, hand-written parsers, or both, as long as the promises hold.

Sources studied: `ic: platform/core-api/src/com/intellij/psi/` (`PsiElement.java`, `PsiFile.java`, `PsiManager.java`,
`FileViewProvider.java`, `PsiReference.java`, `PsiPolyVariantReference*.java`, `PsiReferenceContributor.java`,
`PsiReferenceProvider.java`, `PsiReferenceRegistrar.java`, `PsiReferenceService.java`, `ResolveResult.java`,
`SmartPointerManager.java`, `SmartPsiElementPointer.java`, `PsiTreeChangeEvent.java`, `PsiTreeChangeListener.java`,
`PsiNamedElement.java`, `PsiNameIdentifierOwner.java`, `TokenType.java`), `ic: platform/core-api/src/com/intellij/lang/`
(`ParserDefinition.java`, `PsiBuilder.java`, `SyntaxTreeBuilder.java`, `PsiParser.java`),
`ic: platform/core-api/src/com/intellij/psi/tree/IElementType.java`,
`ic: platform/core-impl/src/com/intellij/psi/impl/` (`BlockSupportImpl.java`, `PsiDocumentManagerBase.java`,
`PsiModificationTrackerImpl.java`), `ic: platform/core-impl/src/com/intellij/psi/impl/source/PsiFileImpl.java`,
`ic: platform/core-impl/src/com/intellij/psi/impl/source/tree/ASTShallowComparator.java`,
`ic: platform/core-impl/src/com/intellij/psi/impl/source/resolve/ResolveCache.java`,
`ic: platform/core-impl/src/com/intellij/psi/impl/file/impl/FileManagerImpl.java`,
`ic: platform/core-impl/src/com/intellij/psi/SingleRootFileViewProvider.java`,
`ic: java/java-psi-impl/src/com/intellij/psi/impl/compiled/ClsClassImpl.java`, `ClsFileImpl.java`,
`ic: java/java-impl/src/com/intellij/psi/impl/JavaPsiImplementationHelperImpl.java`,
`ic: platform/util/resources/misc/registry.properties`.

## 1. Model

### 1.1 Two layers: syntax tree (AST) and typed elements (PSI)

* The **syntax tree** is a lossless tree of nodes. Every character of the file belongs to exactly one *leaf*
  node. Whitespace and comments are leaves too; they are siblings of the code around them, not metadata.
  Nodes have an *element type* (an interned identity), a text range, children, and a parent.
* **Typed elements (PSI)** are objects layered over nodes. The element type of a node alone decides which kind
  of element is created for it (the creation function must not look at text or context). Elements add typed
  accessors (`name_identifier()`, `parameter_list()`), references, and editing operations. Leaves and composite
  nodes both have an element.
* An element can also exist **without a node** when it is backed by a stub (see spec 03) or is synthesized
  (light elements, section 6).

### 1.2 Files and view providers

A **file** is the root element of one tree. A **view provider** manages all trees of one virtual file:

* single-language files: one tree whose language is the file's language;
* multi-language files (templates such as HTML with embedded server code): several trees over the same
  text, one per language, plus one *base* language that decides what "the file" is for lookups;
* the *base* tree is returned for "tree of this file or document" queries; other trees are reached through the provider.

The view provider is also the owner of the link to the document and of the "physical" flag:

* **Physical** (events enabled): the provider is for a real file of the project. Elements of physical trees fire tree
  change events and bump global modification counters, require read access, and are subject to commit rules.
* **Non-physical**: copies, fragments, previews, in-memory files. No events, no global counters, no document commit.

### 1.3 Element lifecycle

```
(no tree) --find_file / get_psi--> loaded tree --edit/commit--> updated tree (many elements keep identity)
                                       |   ^                          |
                                       |   +---- resurrected ---------+   (only if recreation would be equivalent)
                                       +---- delete / type change / full reload --> invalid (final)
```

* A tree is created lazily when first requested, is held by weak/soft references plus whatever strong
  references callers hold, and may be dropped and rebuilt at any time when memory is tight; elements that
  were handed out remain valid as long as the *same* tree is alive.
* **Valid** means: reachable from a live tree of a live file in a live project. Invalid is final.
* Elements become invalid when: they are deleted through the tree; they are replaced by an incremental reparse
  (the new tree differs there); their file is deleted, moved to a different language/type, or fully reloaded;
  the project closes.

## 2. Requirements: tree shape and element contract

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-PSI-001 | MUST | The concatenation of the text of all leaves of a file's tree equals the file's (committed) text exactly, byte for byte, including whitespace and comments. | For every file in a test corpus, `leaves.map(text).join() == file_text`. |
| PLAT-PSI-002 | MUST | For every element, its text range lies within its parent's range, siblings are ordered and non-overlapping, and children of a node cover the node's whole range without gaps. | Property test over corpus: no gap or overlap anywhere in the tree. |
| PLAT-PSI-003 | MUST | Whitespace and comment tokens are present as leaves with distinguished element types (a shared whitespace type, language-defined comment types). Their placement relative to declarations follows *edge-binding* rules: leading doc comments and annotations/modifiers belong to the declaration they describe; trailing same-line comments belong to the preceding declaration only if the language says so. | The text range of a documented Java method includes its doc comment; of an undocumented one it starts at the first modifier. |
| PLAT-PSI-004 | MUST | A parse never fails and never panics. Syntax errors appear as **error elements** (a distinguished element type with a human-readable description) at the position of the error; the surrounding structure is still built as sensibly as the grammar allows. A character that the lexer cannot classify becomes a bad-character leaf. | Parse `class A { void f( }`: tree has an error element with a description, the class and method elements still exist. |
| PLAT-PSI-005 | MUST | Only the *first* of several consecutive error markers at the same position appears in the tree; error elements never overlap other error elements. | Parser emits two errors at one position: tree has one error element. |
| PLAT-PSI-006 | MUST | Element creation is a pure function of the node's element type (and language). The same element type never yields two element kinds depending on text. | Parse the same source twice; same type to element-kind mapping. |
| PLAT-PSI-007 | MUST | Element navigation: parent, first/last child, next/previous sibling, children, containing file, text, text length, text range in file, text range in parent, offset in parent, leaf at offset, reference at offset. Parent of a file element is its directory (or none). `containing_file` of a detached element is none. | API conformance tests for each accessor on a sample tree. |
| PLAT-PSI-008 | MUST | Text range and containing-file lookups cost O(depth) at worst and may not allocate the whole subtree text. Obtaining the text of a composite is O(size) and is documented as expensive. | A benchmark for `text_range` on a depth-200 tree stays under 2 microseconds per call. |
| PLAT-PSI-009 | MUST | Element identity: two elements are the same element iff they are the same object/handle. Equality for *semantic* purposes (source class vs compiled class, copy vs original) goes through `are_elements_equivalent(a, b)` which handles invalid inputs, null/none, and source-versus-library equivalence. | A class from a source file and the same class from a jar compare equivalent but not identical. |
| PLAT-PSI-010 | MUST | Element types are interned and have a stable language. The registry is bounded: IntelliJ allows at most 15,000 element types across all languages together; Archipel must either keep a per-language table (and say so) or enforce an equivalent bound with a clear startup error. | Register more types than the bound: startup reports which plugin overflowed; it does not corrupt ids. |
| PLAT-PSI-011 | MUST | Token sets are immutable sets of element types with O(1) membership. They exist for at least: whitespace, comments, string literals (for "search in strings" and spell-checking), identifiers, and language-specific groups used by the parser. | `comments.contains(t)` for 1M tokens runs without allocation. |
| PLAT-PSI-012 | MUST | The base element kinds every language gets: file, whitespace, comment, error element, leaf. Marker interfaces: *named element* (has a name and can be renamed), *name-identifier owner* (name is an identifier token; offset of caret on navigation is the identifier's offset), *navigatable*, *literal value*, *language-injection host*. | A declaration element reports its name and the range of its name identifier. |
| PLAT-PSI-013 | MUST | `text_offset(element)` is the offset where the caret goes when navigating to the element: for named elements, the name identifier; otherwise the start of the element. | Navigating to a Java method puts the caret on the method name, not on the first modifier. |
| PLAT-PSI-014 | MUST | Visiting: a visitor pattern with `visit(element)` and `visit_children`; recursive walking variants with and without a tree-depth guard; walking must be iterative or depth-safe (a tree can be thousands of levels deep). | Walk a generated 20,000-deep tree without stack overflow. |
| PLAT-PSI-015 | SHOULD | `user_data` on elements and files (typed keys); *copyable* user data is copied when an element is copied. | Put copyable data, copy element: copy has it; plain data is not copied. |
| PLAT-PSI-016 | SHOULD | Every element exposes its **language**; in multi-language files this is the language of the element's tree or of its own type. | A Java expression inside a template file reports Java. |

## 3. Requirements: files, view providers, document link

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-PSI-020 | MUST | `find_file(virtual_file)` returns the base-language file for physical, valid, non-directory files with a language; none for directories, invalid files, binary files without a reader, and in a default/dummy project. `find_directory` returns a directory element for directories. | Unit tests for each exclusion. |
| PLAT-PSI-021 | MUST | A file element reports: virtual file (none for in-memory-only), containing directory, name, modification stamp (= stamp of the last committed text), language, file type, original file (itself unless it is a copy), whether it is physical, its view provider, and its document. | API conformance tests. |
| PLAT-PSI-022 | MUST | A **copy** of a file (for completion, speculative analysis) is a non-physical file linked to its original. Resolve inside the copy resolves to elements of the copy where the target is in the copy, and to the real program otherwise. Edits to the copy never touch the original or fire events. | Insert a dummy identifier in a copy, resolve there; original tree and document untouched. |
| PLAT-PSI-023 | MUST | A file can be created from text (name, file type or language, text) without any file on disk, physical or not. Used by refactorings, quick fixes and tests. | Create a Java file from text, resolve a reference in it against the project. |
| PLAT-PSI-024 | MUST | **Code fragments**: a non-physical file whose context element supplies scope (expression typed in a condition dialog, injected snippet, debugger watch). Resolve walks the fragment, then continues from its context element. | A fragment with context inside method `m` resolves local variables of `m`. |
| PLAT-PSI-025 | MUST | A file larger than the intelligence limit (spec 01, PLAT-VFS-034) gets a *large plain-text* file with no tree structure beyond one leaf; its tree is never parsed. | Open a 3 MB `.java`: the file element has one leaf and no declarations. |
| PLAT-PSI-026 | MUST | Binary files without decompiler produce a binary file element with no text. Binary files with a registered reader (class files) produce a normal compiled file element (section 7). | `.png` gives a binary element; `.class` gives a compiled file element. |
| PLAT-PSI-027 | MUST | The PSI manager exposes: `find_file`, `find_directory`, `find_view_provider`, `are_elements_equivalent`, `reload_from_disk(file)`, `drop_caches()`, `drop_resolve_caches()`, `is_in_project(element)`, a **batch mode** runner (memory is released eagerly while visiting many files), and `modification_tracker`. | `reload_from_disk` replaces the document text and file tree; open editors update. |
| PLAT-PSI-028 | MUST | `is_in_project(element)` is true for elements of files in project *sources* (content) and false for library elements. | A library class is not in project; a project class is. |
| PLAT-PSI-029 | SHOULD | A view provider factory contribution can supply a custom provider per file type or language (multi-root files, template files). | A template language registers a provider with two roots; `get_all_files` returns both with the base first. |
| PLAT-PSI-030 | MUST | Tree access requires read access to the project model; tree modification requires write access and a physical-checked environment. Violations are errors in debug builds. | A background thread without a read section touching a tree fails a debug assertion. |
| PLAT-PSI-031 | MUST | Holding a file's AST is optional: the platform may keep only a stub (spec 03) for files that are not open, and rebuild the AST on demand. Elements handed out from the stub keep their identity when the AST loads (stub-to-AST switch) if the implementation has stub-backed elements. | Query a class via the stub index, then call a method that needs its body: the same element object is returned. |

## 4. Requirements: parsing contract and incremental update

### 4.1 Lexer, parser, tree builder (what a language supplies)

* A **lexer** turns the whole text into tokens `(type, start, end)`; it is *restartable*: it can start in the middle
  from a saved state and must produce the same tokens a full run would (used by the highlighter for incremental update).
* A **parser** receives a **tree builder**: a cursor over non-whitespace, non-comment tokens plus a marker API.
  Markers define node ranges: start a marker at the current position, advance, then *done* it with an element type,
  *drop* it, roll it back (restoring the lexer position), *precede* it with a new marker that starts before it,
  or complete it with an error message. Whitespace and comment tokens are skipped automatically by the builder and
  attached to nodes by the edge-binding rules (PLAT-PSI-003).
* The builder can look ahead and behind by token count, remap the current token's type (and restore on rollback),
  and give a collapsed leaf for a whole range ("done and collapse").
* A **file node type** names the root. A **lazy-parseable** element type (a *chameleon*) is a leaf-looking node whose
  children are produced only when first asked for (typical: code blocks, doc comments); its text is lexed but not parsed.
* A **reparseable** element type declares it can re-parse just its own text after an edit and say whether the result
  is a valid replacement.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-PSI-040 | MUST | A language registers a *parser definition*: lexer factory, parser factory, file node type, token sets for whitespace, comments and string literals, element factory (node to element), file factory (view provider to file), spacing rules between tokens (optional), and optional whitespace re-parse. | A language lacking any mandatory part is rejected at registration with a message naming the part. |
| PLAT-PSI-041 | MUST | Whitespace set should contain only the shared whitespace type unless the language truly needs more. | Lint on language registration. |
| PLAT-PSI-042 | MUST | A marker that is neither done nor dropped when the tree is requested is a parser bug: the platform reports an "unbalanced markers" error with the language and file, then produces a degenerate tree (error leaf for the whole file) rather than crash. | Feed a parser that leaks a marker: the editor shows the file as plain text with an internal-error notice. |
| PLAT-PSI-043 | MUST | Lexer restart contract: for every position at which the lexer reports state `S`, starting a fresh lexer at that position with state `S` yields the same remaining tokens. | Property test: for random edits, relex from a restart point equals full relex. |
| PLAT-PSI-044 | MUST | Chameleons parse on first child access and are parsed under read access; asking a chameleon for its *text* or *length* does not parse it. | Count parser calls: file text length and text of a method do not parse its body. |
| PLAT-PSI-045 | SHOULD | The parser may produce a *light tree* (cheap, no PSI possible) for indexing and for stub building; it must be structurally identical to the full tree for the stubbed parts. | Stub from light tree equals stub from full tree on the corpus. |
| PLAT-PSI-046 | MUST | Parsing a file larger than the intelligence limit is never attempted. Parsing is bounded in depth: nesting deeper than the **depth limit** (default 1000 levels) disables incremental reparse for that file (full reparse only) and marks the file "too deep". | A 5000-deep nested expression does not overflow the stack and the file keeps updating via full reparse. |

### 4.2 Incremental reparse and tree merge

Algorithm (observable behavior; any implementation giving the same promises is fine):

1. The commit gets the *changed range* (old text vs new text, as one range covering all uncommitted edits).
2. If the file is flagged "do not reparse incrementally" (bulk update; whole-text replace of a document longer than
   100,000 characters; depth limit exceeded), go to step 6.
3. Find the leaves at the changed range boundaries and their lowest common ancestor.
4. If a boundary leaf is of a *reparseable leaf* type and the change covers exactly its edge, ask the type to
   re-lex that leaf from the new text. Success replaces just that leaf.
5. Otherwise walk up from the common ancestor: for each ancestor whose type is *reparseable*, ask it to re-parse its
   own new text; accept the first result that the type itself validates (valid replacement) and that parses to the
   *same type*. Stop at the file node. If no ancestor accepts, go to step 6.
6. **Full parse** of the new text into a fresh tree.
7. **Merge**: compare the old subtree (or old file) with the new tree top-down and replace only what differs. Two nodes
   *match* if their types are equal and (for leaves) their text is equal, (for error elements) their descriptions are
   equal, and (for composites) their structural hashes are equal; matching subtrees keep their **old objects**,
   non-matching subtrees are replaced by the new ones. The merge yields a list of tree-change events.
8. Apply the merge under write access, fire tree-change events (section 5), update the file's committed stamp.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-PSI-050 | MUST | After a commit the tree is **equal in shape and text** to a full parse of the committed text (same types, same text for every node), no matter which path was taken. | Differential test: N random edits on corpus files; at each step compare to full parse. |
| PLAT-PSI-051 | MUST | Elements in subtrees the merge judged unchanged **keep identity and validity**; elements in replaced subtrees become invalid. | Add a statement at the end of a method: elements of earlier statements, of other methods, and of the class remain valid and are the same objects. |
| PLAT-PSI-052 | MUST | A one-character edit inside a leaf token that is not reparseable and not inside a reparseable composite results in at worst a full parse + merge of the file; it never invalidates more than the nodes that actually differ. | Edit a comment: only that comment element is replaced. |
| PLAT-PSI-053 | SHOULD | Languages can supply a custom comparator that tells the merge two nodes are equal/different in ways the generic comparison cannot (for example, ignoring formatting-only differences in a lazily-parsed node). | A comparator marks reformatted but equivalent code blocks equal; elements keep identity. |
| PLAT-PSI-054 | MUST | The merge is cancellable and runs off the UI thread in the asynchronous commit; only applying the result needs write access and must be short. | Cancel a merge on a 5 MB-equivalent tree; the old tree stays fully valid. |
| PLAT-PSI-055 | SHOULD | When the changed range is replaced wholesale (the node cannot be matched at all), emit a single "replaced" event for that node instead of per-child events. | Replace a whole class: one replaced event. |
| PLAT-PSI-056 | MUST | Whole-file reload (external change that is not mergeable, file type/language change) drops the entire tree (all elements invalid), then rebuilds lazily. | After external reload, an old element's `is_valid()` is false. |

## 5. Requirements: change notification

Tree-change events exist for features that must react to specific edits (usually they should instead depend on
modification counters, spec 05). They are *difficult to use correctly*, and the platform says so: precise events
depend on implementation details (which node was reparsed, whether the tree was loaded).

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-PSI-060 | MUST | Event kinds: before/after child added, before/after child removed, before/after child replaced, before/after children changed, before/after child moved, before/after property changed. Each event names parent, child (or old/new child), the file, offset and old length where known, and for property changes the property name with old and new value. | Unit-check one of each. |
| PLAT-PSI-061 | MUST | Property-change events with the following *generic* property names exist and are fired as documented: file name changed, directory name changed, writable changed, roots changed (project model), file types changed (any file's type may have changed), "unloaded PSI changed" (something changed where no tree was loaded, so no specific event is possible). Listeners that cannot be precise drop their caches on the last three. | Change roots: a listener receives a roots event. |
| PLAT-PSI-062 | MUST | "Before" and "after" events are *not guaranteed to pair*: several "before" events may be followed by one "children changed". Listeners must not rely on pairing. | The platform's own listeners are written to this rule; a test fires unpaired events. |
| PLAT-PSI-063 | MUST | Events are fired on the write thread inside the write section that changed the tree, for physical trees only. Listeners must not modify PSI or documents, must not call commit, and must not expand lazily parsed nodes. | A listener calling `commit_document` returns the error `reentrant commit`. |
| PLAT-PSI-064 | SHOULD | A listener flavor that may run on any thread (for pure-cache listeners) exists next to the write-thread one. | A listener registered as "any thread" receives events with no UI dependency. |
| PLAT-PSI-065 | MUST | Listener registration is tied to a lifetime (removed automatically when the owner disposes). Failure in one listener is isolated. | Dispose an owner; its listener receives nothing more. |
| PLAT-PSI-066 | MUST | A **pre-processor** contribution runs before listeners on every event and may bump language-specific counters (spec 05). The platform's own modification tracker is one of them. | After any physical tree change `modification_count` is strictly greater. |
| PLAT-PSI-067 | MUST | "Any PSI change" has two phases: *before physical change* (used to clear resolve caches) and *after*. Resolve caches are cleared in the *before* phase so no reader can see a stale entry across the change. | A resolve cached before an edit is not returned after it. |

## 6. Requirements: validity, invalid access, smart pointers

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-PSI-070 | MUST | An element is valid iff it belongs to a live tree. Using an invalid element (other than checking validity or identity) returns an **invalid-element error** carrying the reason when tracked (deleted, replaced by reparse, file deleted, file type changed, project closed) and, in debug builds, where it was invalidated. | Hold an element, delete its file, call `parent()`: typed error with reason "file deleted". |
| PLAT-PSI-071 | MUST | A "possibly invalidated" file is revalidated lazily: after a rename, move or file-type event, the file is marked possibly-invalid; on next access, the platform re-creates the view provider for the file; if the result is equivalent (same language set and same file type) the **old tree is kept and the file is valid again**, otherwise the old tree is invalid. | Rename `A.java` to `B.java`: the file element is still valid and the same object. |
| PLAT-PSI-072 | MUST | Elements never become valid again after a *tree-level* invalidation (deleted/replaced). Only file-level possible invalidation can revalidate, once, under PLAT-PSI-071. | Replace a node by reparse: its old elements stay invalid forever. |
| PLAT-PSI-073 | MUST | Reading from a valid element after the document changed but before commit is allowed; it returns the *last committed* structure. Modifying such an element is an error if its document is uncommitted (commit first). | Edit the document; read element text: old text. Call `replace` on it: error `uncommitted`. |
| PLAT-PSI-074 | MUST | Code that obtains elements in one read section and uses them in another **must** re-validate or use smart pointers; any write section in between may invalidate them. The platform documents this and provides an `ensure_valid` helper that returns the invalid-element error. | A test performs a write between two reads and expects invalid or a pointer re-resolution. |
| PLAT-PSI-075 | MUST | **Smart pointers** survive edits: `create_pointer(element)` yields a handle whose `dereference()` returns the element in the *current* tree that corresponds to the original, or none if it no longer exists. Pointers to files and ranges exist as well. | Insert text before an element and reparse: the pointer returns the same logical element. Delete the element: none. |
| PLAT-PSI-076 | MUST | A smart pointer reports the element's range in the current document (it follows uncommitted edits like a range marker, may become none) and its range in the last committed file (stable until commit). | While uncommitted, `range()` and `psi_range()` can differ; after commit they are equal. |
| PLAT-PSI-077 | MUST | Smart pointers do not keep elements alive by themselves (they can be used to hold many references cheaply) and must be cheap to create for thousands of elements in one file. | Create 50,000 pointers in one file under 100 ms and 20 MB. |
| PLAT-PSI-078 | SHOULD | Comparing two pointers is cheaper than dereferencing both and needs no read access beyond what it takes internally. | Pointer equality on 10,000 pairs without trees loaded. |
| PLAT-PSI-079 | SHOULD | Explicit disposal of a pointer is allowed but not required. | n/a |
| PLAT-PSI-080 | MUST | When a file is fully reloaded and the pointed element's counterpart exists structurally (same file, same ancestry kind path, same name), implementations *may* re-find it; they must never return an element of a different kind or name than the original. | Property test: random edits; `dereference()` never returns an element of another kind. |

## 7. Requirements: navigation elements, library classes versus sources

`navigation_element` answers "where should the editor go when the user navigates to this element?". `original_element`
is its inverse: "which element in the project's sources or class path corresponds to this one?".

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-PSI-090 | MUST | Every element has `navigation_element()` (default: itself) and `original_element()` (default: itself). Navigation always goes to `navigation_element()`'s file and `text_offset`. | Go to declaration on a project class navigates to the class name. |
| PLAT-PSI-091 | MUST | For an element of a **compiled** file (class file), the navigation element is the matching source element if the class has attached sources; otherwise itself (the editor then shows the decompiled mirror text). | With a sources jar attached, Go to Declaration on `ArrayList.add` opens `ArrayList.java`; without, the decompiled view. |
| PLAT-PSI-092 | MUST | Source lookup for a compiled class file: take the *source file name* recorded in the class file and the class's package; search the source roots of the libraries that contain the class (libraries sorted by name), then of the SDKs that contain it; the first root where `<package path>/<source file name>` exists and parses to a class-owning file wins. For classes in versioned (multi-release) archives look first in the versioned source path. Inner classes follow their outer class's source mirror by name. If the parent is not mapped, neither is the child. | `org.foo.Bar$Baz` maps to inner class `Baz` of `Bar.java` in the first library with sources. |
| PLAT-PSI-093 | MUST | A contribution can override navigation for compiled files and classes (custom navigation policy), consulted first; failures due to "index not ready" are swallowed and fall through to the default. | A policy returning a Kotlin source for a compiled Kotlin class is honored. |
| PLAT-PSI-094 | MUST | The navigation-file lookup for a compiled file is cached and invalidated by: the file, the target file, and root changes of the project model. | Attach a sources jar: navigation targets update without restart. |
| PLAT-PSI-095 | MUST | A compiled file has a **mirror**: decompiled text (or generated signature text) parsed as a normal source file, whose structure must match the compiled structure element by element; the mirror is lazy, built on first text access, and mirror mismatch is an internal error, not a crash. | Open decompiled view of a class file; every member of the compiled class has a mirror member. |
| PLAT-PSI-096 | SHOULD | Navigation items implement a presentation: text, location string (container/file/module), icon. Presentation of the same element in two lists (Find Usages, Search Everywhere, structure) is consistent. | n/a, reviewed in UI specs. |
| PLAT-PSI-097 | SHOULD | `navigate(requestFocus)`, `can_navigate`, `can_navigate_to_source` exist on navigation items; compiled elements without sources report `can_navigate_to_source = false` but still navigate to the decompiled view. | n/a |

## 8. Requirements: references and resolve

### 8.1 Reference contract

A **reference** belongs to one *host element* and covers a sub-range of it. It can resolve to one target or, for
poly-variant references, to several candidates with a validity flag each. References are what Go to Declaration, Find
Usages, Rename, highlighting of unresolved symbols, completion and inspections are built on.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-PSI-100 | MUST | A reference exposes: `element()` (host), `range_in_element()`, `absolute_range()` (range in the file), `resolve()` (target or none), `canonical_text()` (name independent of imports/context, for example the fully qualified class name), `handle_element_rename(new_name)`, `bind_to_element(target)`, `is_reference_to(element)`, `variants()` (candidates for completion), `is_soft()`. | Conformance test against a toy language implementation. |
| PLAT-PSI-101 | MUST | `resolve()` returns a **valid** element or none; it returns none when unresolved *or* when ambiguous. It never returns an invalid element and never panics. | Resolve an overloaded call with two equally applicable candidates: none; `multi_resolve` returns both. |
| PLAT-PSI-102 | MUST | A poly-variant reference additionally has `multi_resolve(incomplete_code)` returning results with `(element, is_valid_result)`. With `incomplete_code = true` the code is considered being typed and extra *invalid* results may be returned to enable completion/quick fixes. `resolve()` of such a reference is the element of the single result, only if there is exactly one result. | Static call to an instance method: one *invalid* result in `multi_resolve`; unresolved-reference error is not reported; a "non-static from static context" error is. |
| PLAT-PSI-103 | MUST | A **soft** reference never produces an unresolved-reference error when it fails to resolve (text in an XML attribute that merely looks like a class name). A **hard** reference does. | A soft reference to a non-existent class: no error highlighting. |
| PLAT-PSI-104 | MUST | `is_reference_to(element)` compares the resolved target(s) to `element` with `are_elements_equivalent` (not identity), so a source class and its compiled twin match. For poly references it is true if *any* valid result matches. | A reference resolving to a library class source is a reference to the compiled class. |
| PLAT-PSI-105 | MUST | A host element can carry several references (each a different sub-range, such as the segments of a qualified name; a string literal with several class names). References of one host must not overlap unless they are alternative interpretations of the same range. | `a.b.C` has one reference per segment with ranges excluding dots. |
| PLAT-PSI-106 | MUST | `element.get_reference()` returns the first reference or none; `element.get_references()` returns all (host-provided); `reference_service.get_references(element, hints)` returns host references **plus contributed references** for hosts that opt in as contributed-reference hosts. | A contributed reference on a string literal appears in the service result but not in the bare element accessor of a non-opted host. |
| PLAT-PSI-107 | MUST | The reference at a file offset is the reference whose absolute range contains the offset, preferring the most specific; `reference_at(offset)` exists on file and element. At a boundary between two references the one *starting* at the offset wins. | Caret between `a` and `.b`: reference of `a` for offset at end of `a`; of `b` after the dot. |
| PLAT-PSI-108 | SHOULD | Rename support: `handle_element_rename` returns the new underlying element (which may differ from the old one), may throw an editing error (unsupported), and must keep the reference resolving to the renamed element. | Rename a variable: all references rewritten and still resolve. |
| PLAT-PSI-109 | SHOULD | `bind_to_element(target)` rewrites the reference so that it resolves to `target`, adding imports/qualifiers as the language demands. | "Create class from usage" then bind: reference resolves to the new class. |

### 8.2 Contributed references

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-PSI-110 | MUST | A *reference contributor* registers *providers* for **locations described by element patterns** (for example "string literal that is an argument of method X", "XML attribute value of attribute Y in tag Z"). Contributors run **once per project** (at first use); providers are asked per host element. | Register a provider for string literals in `@Value("...")`; it runs for matching literals only. |
| PLAT-PSI-111 | MUST | Provider priorities: default 0, higher +100, lower -100 (any real number in between). Merge rule for one host element: collect the non-empty results of all matching providers grouped by priority; **all references of the highest priority group are kept as they are**; a reference from a lower group is kept only if its range does not intersect any reference already kept from a higher group. The lowest effective priority is -100. | Two providers on the same literal at priorities 0 and 100 covering the same range: only the +100 references remain; a priority-0 reference on a disjoint sub-range of the same literal also remains. |
| PLAT-PSI-112 | MUST | A provider can be asked *cheaply* whether it could possibly return references for given hints (a search target and/or offset) before the pattern is evaluated; Find Usages uses this to skip providers that cannot yield the searched target. | A provider returning "no" for target T is never invoked when searching usages of T. |
| PLAT-PSI-113 | MUST | A **language that wants contributed references** must opt in by routing its elements' reference accessor through the registry (it is not automatic). Hosts opt in via a marker. | A language without opt-in ignores contributors; documented in the language-extension spec. |
| PLAT-PSI-114 | SHOULD | Hints: a *highlighted references only* hint exists so the editor highlight pass asks only providers that produce references that should be underlined. | Highlight pass does not instantiate providers that only support completion. |
| PLAT-PSI-115 | MUST | Providers run under read access, are side-effect free, thread-safe, must not store state across calls, and honor cancellation. | Concurrent calls on one provider instance are safe in a stress test. |

### 8.3 Resolve caching and recursion

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-PSI-120 | MUST | The platform provides a **resolve cache**: results of `resolve`/`multi_resolve` are cached per reference object, separately for (physical vs non-physical) x (complete vs incomplete code) x (single vs poly), eight partitions in total. A different flag never returns another partition's result. | Resolve the same reference with `incomplete_code` true and false: two independent computations. |
| PLAT-PSI-121 | MUST | The cache is cleared **before** every physical PSI change (non-physical partitions only before non-physical changes) and on low memory. Keys are weak, values soft; "no result" and empty arrays are cached strongly (they are tiny). | After any edit, no stale resolve result is observable. |
| PLAT-PSI-122 | MUST | Resolve results are cached only if the computation was not influenced by recursion-prevention (a recursive request that was cut short returns a provisional result that must not be cached). | A circular type inference returns a provisional result for the inner call and does not poison the cache. |
| PLAT-PSI-123 | MUST | Resolve requires read access for physical references and honors cancellation on entry (each cache access is a cancellation point). Cancellation during computation leaves no partial entry. | Cancel mid-resolve; the next resolve recomputes cleanly. |
| PLAT-PSI-124 | MUST | Every element a resolver returns is checked valid before caching; an invalid result is an internal error that names the reference class. | A buggy resolver returning a deleted element is reported with its class name. |
| PLAT-PSI-125 | MUST | Resolve is **idempotent**: two successive resolves of the same reference with no intervening change return equivalent results (equal targets in the same order). Debug builds randomly re-compute a cached result and compare. | Self-check hook enabled in tests detects a non-idempotent resolver. |
| PLAT-PSI-126 | SHOULD | Offer resolve with an explicit *containing file* argument to avoid recomputing it up the tree. | Benchmark shows fewer tree-up walks. |
| PLAT-PSI-127 | MUST | Resolve in dumb mode (spec 04): references that need indexes return none or throw the typed "index not ready" error **which callers on the UI path must catch**; analysis features check dumb mode before resolving. | In dumb mode a Java reference that needs the class index yields the typed error, not a wrong answer. |

### 8.4 Scopes for resolve and declarations processing

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-PSI-130 | MUST | Name resolution inside a language walks **up the tree** from the reference's place: each ancestor is asked to *process its declarations* with a processor (visitor of declarations) and the child it came from (so it can skip what was already processed), until the processor says stop. An element may redirect the walk by naming a different *context* parent (code fragments, injected code). | Local variable shadowing a field resolves to the local. |
| PLAT-PSI-131 | MUST | The **resolve scope** of a file is the search scope in which names from other files are looked up. Default (source file in a module): the module with its dependencies and libraries, including test sources iff the file is a test file. Library file: the scopes of all modules that use that library, preferring the same library. Non-project file: everything plus the file itself. Contributions can *replace* (first non-null wins) or *enlarge* (union) the scope per file. See spec 07 for scope semantics. | A production file does not see test classes; a test file sees production and test. |
| PLAT-PSI-132 | MUST | The **use scope** of an element is where references to it can occur. Platform rule for a *file or directory*: for a file in a module, the module plus all modules that depend on it (production-only unless the file is test code, in which case the test-aware variant); for a file in a library, all modules using the library plus library files; for a scratch file only that file; for an injected fragment its host file; otherwise everything. Java member rule (on top): public/protected: the file's use scope, narrowed to the containing class's use scope when no subclass can widen it (final class, constructor, or a private class that is not inherited locally); package-private: the package intersected with the file's use scope; private: the top-level class; members of local or anonymous classes: the enclosing statement or class (widened if the member overrides a supertype method and is visible); implicit classes: the class. Contributions can enlarge a use scope. | Usages of a private field are searched only in its top-level class; of a public class in dependent modules only, not unrelated ones. |
| PLAT-PSI-133 | MUST | Resolve and use scopes are cached per file and cleared on any physical PSI change, on contribution changes, and on project-model changes. | After adding a module dependency, resolve scope reflects it. |
| PLAT-PSI-134 | SHOULD | The *original file* rule: a copy of a file resolves with the scope of its original, plus the copy file itself. | Completion copy sees the same classes as the original. |

## 9. Requirements: PSI modification (editing through the tree)

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-PSI-140 | MUST | Operations: add (child, before anchor, after anchor), add range, delete, delete child range, replace. Each returns the element **actually inserted** (the original or a copy). Each fails with an *incorrect operation* error (not a panic) if unsupported or not possible (read-only file, invalid structure). | Add a method to a class; returned element is the inserted method. |
| PLAT-PSI-141 | MUST | Tree modification runs in a write section; at the end the change is written to the document in **one** document change group (so undo is one step). Document and tree are in sync when the section ends. | Rename via PSI then undo once: reverted. |
| PLAT-PSI-142 | MUST | Whitespace between inserted and neighboring elements is managed according to the language's spacing rules (parser definition), without running the full formatter; callers that want formatting call the formatter on the result range. | Insert an `import`: separated by a newline, not glued. |
| PLAT-PSI-143 | MUST | Moving an element between files or copying with `copy()` produces **non-physical** copies; adding such an element into a physical tree makes the inserted element physical (a *copy* is inserted and returned). | Add an element from a fragment: returned element is physical and not the fragment's. |
| PLAT-PSI-144 | SHOULD | Operations check writability of the file first (read-only status, version-control veto) and report a single user-visible reason. | Editing a read-only file fails with a read-only reason. |

## 10. Requirements: light elements, language injection, other trees

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-PSI-150 | MUST | **Light elements**: synthetic elements without a node (for example a Java class generated from a Kotlin class, a getter produced by an annotation processor). They behave like elements (name, parent, text range of their *origin*, resolve) and are **equal iff their origin and kind are equal** so repeated computation yields equal objects. | Two calls generating the same light method return equal objects. |
| PLAT-PSI-151 | MUST | A Java-side **augment contribution** can add members (fields, methods, inner classes) to a class on demand, with an optional name hint for efficiency, provide inferred types, extension methods, transform modifier queries, and flag that a field initializer can be changed by external tools. Only dumb-aware augmenters run during indexing. | An augmenter adds `getName()` to an annotated class; resolve of `foo.getName()` works; during indexing it is not called. |
| PLAT-PSI-152 | SHOULD | **Language injection**: a contribution decides, for a host element (a string literal, an XML text), a *set of places* and a language; the platform builds a small non-physical file whose text is the concatenation of prefixes, place texts (with escapes decoded) and suffixes. Offsets map both ways. All features work inside the fragment. | SQL in a Java string: completion of table names inside the literal. |
| PLAT-PSI-153 | SHOULD | Injection hosts expose how to **decode and encode** their text (escape handling), and a manipulator for changing their *value range* content, so refactorings can edit injected text. | Rename a symbol inside an injected fragment: the host literal text updates with escapes preserved. |
| PLAT-PSI-154 | LATER | Injected languages are re-evaluated when the host changes; injection results are cached with the host file's modification count. | Edit the host: fragment is rebuilt once. |

## 11. Requirements: modification counts (summary; details in spec 05)

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-PSI-160 | MUST | A project-wide **PSI modification count** increases on every physical tree change, on file add/remove/rename in a loaded directory tree, on file-type changes, on roots changes, and on entering and leaving dumb mode. It does **not** increase for non-physical trees, and not for tree changes that are only "property: writable". | Edit a copy: no change. Edit the file: change. |
| PLAT-PSI-161 | MUST | Per-language trackers exist: increased when any element of that language (or its file) changes, plus when the generic events of PLAT-PSI-061 occur. A tracker for the *any* language increases on every change. | A Kotlin edit increases the Kotlin tracker, not the XML one. |
| PLAT-PSI-162 | MUST | The reference implementation has **no separate "out of code block" counter** in the studied version: the older split between code-block-local and structural modifications was removed, and a legacy entry point that "bumps the out-of-code-block counter" now bumps the single global counter. Archipel may implement a finer optimization (spec 05) but must keep the promises of PLAT-PSI-160 and 161. | Replace any use of a structure-only counter with the global or per-language one without loss of correctness. |

## 12. Notes for implementers (non-binding)

* A tree-sitter based tree has no whitespace leaves. To meet PLAT-PSI-001..003, synthesize whitespace/comment leaves from
  the gaps between nodes (virtual leaves) and implement edge binding on top of the concrete tree. Do not drop comment nodes
  into "extras" that are invisible to the element API.
* Tree-sitter nodes have no identity beyond a tree. PLAT-PSI-051 ("unchanged elements keep identity") is the expensive
  promise of the reference model. Archipel can meet its *purpose* (references to elements survive unrelated edits) with
  smart pointers (PLAT-PSI-075) plus stable *anchors*; however, any API that hands out an element handle to features
  (inspections, daemon results) must say whether the handle is a pointer or a tree-bound node.
* The reference model stores the *text* of types and annotations in stubs and resolves lazily; see spec 03 for what a
  stub must contain so library and project classes can be resolved without parsing method bodies.

## 13. Open questions

1. The Symbol API (declarations and references that are not elements, `own_references` on elements) is present in
   the studied tree beside PSI references; Kotlin and framework plugins increasingly use it. Marked LATER here;
   decide whether Archipel's reference model should be symbol-first from the start.
2. Should Archipel keep per-language element-type tables or one registry with a bound (PLAT-PSI-010)?
3. Versioned (isolated) PSI timelines for background commits exist in the studied tree and are marked experimental
   there; not specified here.

# Language servers and completion: evidence

Reference scene: `09-completion.png` (IntelliJ IDEA Community 2025.2.6.3, java-gradle sample, and
Android Studio 2026.2.1) from `docs/parity/evidence/reference/`. Before this work Archipel IDE showed
"command is disabled: language.complete" for Java and Kotlin files
(`archipel-ide/java-gradle/09-completion-absent.png`).

## Environment

- Linux, Xvfb 1600x1000, `ARCHIPEL_RENDERER=glow ARCHIPEL_NO_UPDATE_CHECK=1`.
- rust-analyzer 1.97.0 (rustup component), Eclipse JDT LS 1.40.0 (`jdtls` on PATH), OpenJDK 21,
  JetBrains kotlin-lsp server 263.6379.0 (from the Open VSX `JetBrains.kotlin-server` bundle).
- `/home/user/samples/java-gradle`: `settings.gradle.kts`, `build.gradle.kts` (application plugin,
  Java 21 toolchain) and `src/main/java/com/example/App.java` as in the reference scene.

## Scenes

| File | Steps | Result |
|---|---|---|
| `status-jdtls-import-progress.png` | Open the project, open `App.java` | jdtls starts lazily on the first Java file. The status bar shows its `$/progress` report ("jdtls: Initialize Workspace 69% (Importing root project …)") with a spinner. |
| `09-java-completion-jdtls.png` | After the import, type `app.` on line 10 and press Ctrl+Space | The popup lists the jdtls members (`clone`, `equals`, …, `greeting`, …) with their signatures. jdtls diagnostics are also shown (squiggle under `app.`, red dot on Problems). |
| `kotlin-server-missing-notice.png` | Open a `.kt` file without a Kotlin server installed | Balloon "Kotlin language server not found" with the install hint, "Copy Install Hint" and "Restart Language Servers". The status bar shows "kotlin: not installed". |

### Kotlin

| File | Steps | Result |
|---|---|---|
| `09-kotlin-completion-kotlin-lsp.png` | `/home/user/samples/kotlin-app` (Kotlin JVM Gradle project), type `greeter.` | JetBrains kotlin-lsp (server build 263.6379.0, `bin/intellij-server --stdio`, linked as `kotlin-lsp`) answers while the status bar still shows "kotlin: Importing (Gradle)". `greeting(name: String)  String` is first, then `equals`/`hashCode`/`toString`. This matches the Android Studio scene's row layout. |

### Completion popup rework (slice 2)

| File | Steps | Result |
|---|---|---|
| `09-java-autopopup-after-dot.png` | Type `app.` (no Ctrl+Space) | The popup opens by itself on the server trigger character. `greeting(String name)  String` comes first, as in the reference: jdtls gives every member the same `sortText`, and the jdtls provider ranks members inherited from `java.lang.Object` after the class's own. The layout matches the reference rows: kind icon, label, grey parameters, right-aligned type, footer hint. |
| `09b-java-filter-prefix.png` | Type `g` | Filtered on the client to `greeting`, `getClass` with the matched `g` in bold. There is no new server request. |
| `09c-java-camel-hump-hC.png` | Replace with `hC` | Camel-hump match `hashCode` (`h` + `C` bold). |
| `09d-rust-autopopup-rust-analyzer.png` | Rust: `message.` in `/home/user/samples/rust-app` | rust-analyzer items with `labelDetails` types right-aligned. |
| `09e-rust-camel-hump-ps.png` | Type `ps` | `push_str` via the `p`…`_s` humps. |

Accepting with Enter inserts the label at the caret. Tab replaces the identifier after the caret
(covered by `archipel-ide-core` completion tests, including `additionalTextEdits` as one undo step).

### Snippets (slice 3)

| File | Steps | Result |
|---|---|---|
| `09g-java-method-parentheses.png` | `app.gr`, Enter, then type `"you"` | jdtls' snippet inserts `greeting()` with the caret inside the parentheses, as in the reference. Typing fills the argument. rust-analyzer is started with `completion.callable.snippets = "add_parentheses"` for the same behaviour. |
| `09f-java-fori-snippet-tabstops.png` | Type `fori`, Enter; type `long`, Tab, `k`, Tab ×3, type a statement | The template is re-indented to the current line. Every variable is boxed, and the current one is selected so typing replaces it. The mirrored `i` changes in all three places at once. Tab/Shift+Tab move between variables, Enter/Tab on the last one goes to `$0` inside the loop body, and Esc leaves the template. |

### Documentation (slice 4)

| File | Steps | Result |
|---|---|---|
| `09h-java-completion-docs-panel.png` | `app.`, Ctrl+Q, then select `finalize` | Ctrl+Q while the popup is open toggles a documentation panel beside it, and the choice is kept. The selected row is resolved (`completionItem/resolve`). The Javadoc is rendered from Markdown: definition header, paragraphs, inline code, links. Deprecated `finalize` is struck through and listed last. jdtls postfix templates (`nnull`, `sysout`, …) are listed after the class's members, as in the reference. |
| `quick-doc-hover-java-string.png` | Rest the mouse on `String` (line 6) | After 600 ms `textDocument/hover` shows the Javadoc of `java.lang.String` with its code examples. It closes when the pointer leaves the symbol and the popup. Ctrl+Q (F1 on macOS) shows the same popup at the caret. |

Accepting an item that was never resolved resolves it afterwards and applies its auto-import
`additionalTextEdits` (jdtls only computes imports during resolve). This is covered end to end by
`crates/archipel-ide-core/tests/fake_lsp_flow.rs`.

### Parameter info (slice 5)

| File | Steps | Result |
|---|---|---|
| `parameter-info-after-completion.png` | `app.gr`, Enter | Completing a call leaves the caret in `greeting(|)` and opens parameter info by itself, with `String name` highlighted, as the reference IDE does. |
| `parameter-info-overloads.png` | Type `String.valueOf(` | `(` is a jdtls signature-help trigger character. All overloads are listed above the call. The active one is bright with its current parameter in bold, the others are grey. The popup moves below the caret when it does not fit above. Ctrl+P (⌘P) opens it explicitly, typing `,` or moving the caret re-queries it, and Esc closes it. |

## Still different from the reference

- jdtls still creates `.project` and `bin/` in Gradle projects (the same as the VS Code Java
  extension). `.classpath` and `.settings` are kept out by passing
  `-Djava.import.generatesMetadataFilesAtProjectRoot=false`.
- No star/arrow relevance markers. Links in documentation are not clickable. Parameter info does not
  narrow the overloads by the arguments already typed (the server decides the active one).
- The popup closes when nothing matches, like the reference. Explicit Ctrl+Space with no results
  reports "No suggestions" in the status bar instead of a popup message.

## Reproduce

```bash
cargo test -p archipel-ide-lsp                                         # fake-server integration tests
cargo test -p archipel-ide-lsp --test real_servers -- --ignored        # rust-analyzer + jdtls, if installed
```

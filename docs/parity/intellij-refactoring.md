# IntelliJ IDEA parity: refactorings

Refactorings from the Refactor menu and Refactor This popup (Ctrl+Alt+Shift+T). Each
refactoring is expected to preview usages, detect conflicts, and be undoable as one
project-wide action.

Sources: IntelliJ IDEA public documentation (<https://www.jetbrains.com/help/idea/>), the predefined Windows
keymap reference (<https://www.jetbrains.com/help/idea/reference-keymap-win-default.html>) and the unified-distribution FAQ (<https://lp.jetbrains.com/intellij-idea-unified-faq/>), which lists
the features that need an Ultimate subscription. Rows were written from public
documentation and observable behavior only; no JetBrains code, artwork, or branding
is reused.

Rows tagged `[Ultimate]` require an IntelliJ IDEA Ultimate subscription. Untagged rows are
available in the free distribution. `Weight` (1-5) is the relative importance used by the
weighted dashboard: 5 = core daily workflow, 3 = significant, 2 = moderate, 1 = minor.
Statuses describe Archipel IDE, conservatively: nothing in this catalog is `Verified`.

| ID | Reference | Feature | Status | Weight | Commands | Tests | Limitations / evidence |
|---|---|---|---|---:|---|---|---|
| IJ-REF-001 | [refactoring-source-code](https://www.jetbrains.com/help/idea/refactoring-source-code.html) | Refactor This popup (Ctrl+Alt+Shift+T) | Missing | 3 | — | — | not started |
| IJ-REF-002 | [refactoring-source-code](https://www.jetbrains.com/help/idea/refactoring-source-code.html) | Refactoring preview and conflict dialogs | Missing | 3 | — | — | not started |
| IJ-REF-003 | [refactoring-source-code](https://www.jetbrains.com/help/idea/refactoring-source-code.html) | Project-wide undo of a refactoring | Missing | 3 | — | — | not started |
| IJ-REF-010 | [rename-refactorings](https://www.jetbrains.com/help/idea/rename-refactorings.html) | Rename symbol (Shift+F6) in place | Partial | 5 | `refactor.rename` (Shift+F6), `refactor.rename.setName`, `refactor.rename.apply`, `refactor.rename.cancel` | `cargo test -p archipel-ide-core refactor`, `cargo test -p archipel-ide-lsp --test fake_server` | LSP `prepareRename` then `rename`; the occurrences in the file (`references`) preview the typed name live, Enter applies the WorkspaceEdit across files (one undo step per file), Escape cancels, servers without prepareRename get a dialog. jdtls 1.40 renamed `greeting` in two files (`evidence/refactoring/01-rename-preview.png`, `02-rename-applied.png`, `03-rename-other-file.png`). Preview tails of changed lines lose syntax highlighting; no rename of comments/strings, no conflict dialog |
| IJ-REF-011 | [rename-refactorings](https://www.jetbrains.com/help/idea/rename-refactorings.html) | Rename file / class / package with references | Missing | 3 | — | — | not started |
| IJ-REF-012 | [rename-refactorings](https://www.jetbrains.com/help/idea/rename-refactorings.html) | Rename in comments and strings, related symbols (tests, getters) | Missing | 1 | — | — | not started |
| IJ-REF-020 | [change-signature](https://www.jetbrains.com/help/idea/change-signature.html) | Change Signature (Ctrl+F6) | Missing | 3 | — | — | not started |
| IJ-REF-021 | [extract-method](https://www.jetbrains.com/help/idea/extract-method.html) | Extract Method (Ctrl+Alt+M) | Partial | 5 | `refactor.extractMethod` (Ctrl+Alt+M) | `cargo test -p archipel-ide-core refactor`, `cargo test -p archipel-ide-lsp --test fake_server` | codeAction `refactor.extract.function`/`.method`; ambiguous results open the intention popup. Verified with the fake server; not yet run on jdtls; no parameter/name dialog |
| IJ-REF-022 | [extract-variable](https://www.jetbrains.com/help/idea/extract-variable.html) | Introduce Variable (Ctrl+Alt+V) | Partial | 5 | `refactor.extractVariable` (Ctrl+Alt+V) | `cargo test -p archipel-ide-core refactor`, `cargo test -p archipel-ide-lsp --test fake_server` | codeAction `refactor.extract.variable`; jdtls 1.40 offers `Extract to local variable (replace all occurrences)` and `Extract to local variable` in the popup (`evidence/refactoring/04-extract-variable.png`, `05-extract-variable-applied.png`); no in-place name editing of the new variable |
| IJ-REF-023 | [extract-constant](https://www.jetbrains.com/help/idea/extract-constant.html) | Introduce Constant (Ctrl+Alt+C) | Partial | 3 | `refactor.extractConstant` (Ctrl+Alt+C) | `cargo test -p archipel-ide-core refactor`, `cargo test -p archipel-ide-lsp --test fake_server` | codeAction `refactor.extract.constant`; verified with the fake server only; no name dialog |
| IJ-REF-024 | [extract-field](https://www.jetbrains.com/help/idea/extract-field.html) | Introduce Field (Ctrl+Alt+F) | Missing | 3 | — | — | not started |
| IJ-REF-025 | [extract-parameter](https://www.jetbrains.com/help/idea/extract-parameter.html) | Introduce Parameter (Ctrl+Alt+P) | Missing | 3 | — | — | not started |
| IJ-REF-026 | [extract-parameter](https://www.jetbrains.com/help/idea/extract-parameter.html) | Introduce Functional Parameter | Missing | 1 | — | — | not started |
| IJ-REF-027 | [extract-into-class-refactorings](https://www.jetbrains.com/help/idea/extract-into-class-refactorings.html) | Introduce Parameter Object | Missing | 1 | — | — | not started |
| IJ-REF-028 | [extract-interface](https://www.jetbrains.com/help/idea/extract-interface.html) | Extract Interface | Missing | 2 | — | — | not started |
| IJ-REF-029 | [extract-superclass](https://www.jetbrains.com/help/idea/extract-superclass.html) | Extract Superclass | Missing | 2 | — | — | not started |
| IJ-REF-030 | [extract-into-class-refactorings](https://www.jetbrains.com/help/idea/extract-into-class-refactorings.html) | Extract Delegate / Extract Class | Missing | 1 | — | — | not started |
| IJ-REF-031 | [extract-into-class-refactorings](https://www.jetbrains.com/help/idea/extract-into-class-refactorings.html) | Extract Method Object | Missing | 1 | — | — | not started |
| IJ-REF-040 | [inline](https://www.jetbrains.com/help/idea/inline.html) | Inline variable / method / constant (Ctrl+Alt+N) | Partial | 5 | `refactor.inline` (Ctrl+Alt+N) | `cargo test -p archipel-ide-core refactor`, `cargo test -p archipel-ide-lsp --test fake_server` | codeAction `refactor.inline`; verified with the fake server only; no usage preview |
| IJ-REF-041 | [inline](https://www.jetbrains.com/help/idea/inline.html) | Inline superclass / class | Missing | 1 | — | — | not started |
| IJ-REF-050 | [move-refactorings](https://www.jetbrains.com/help/idea/move-refactorings.html) | Move class / file / package (F6) | Missing | 3 | — | — | not started |
| IJ-REF-051 | [move-refactorings](https://www.jetbrains.com/help/idea/move-refactorings.html) | Move static members / instance method | Missing | 2 | — | — | not started |
| IJ-REF-052 | [move-refactorings](https://www.jetbrains.com/help/idea/move-refactorings.html) | Copy class / file (F5) | Missing | 2 | — | — | not started |
| IJ-REF-053 | [pull-members-up](https://www.jetbrains.com/help/idea/pull-members-up.html) | Pull Members Up | Missing | 2 | — | — | not started |
| IJ-REF-054 | [push-members-down-dialog](https://www.jetbrains.com/help/idea/push-members-down-dialog.html) | Push Members Down | Missing | 1 | — | — | not started |
| IJ-REF-060 | [safe-delete](https://www.jetbrains.com/help/idea/safe-delete.html) | Safe Delete (Alt+Delete) with usage search | Partial | 3 | `project.safeDeleteUsages`, `project.safeDelete` | `safe_delete_reports_usages_until_forced` (`crates/archipel-ide-core/src/project_view.rs`) | Alt+Delete on a project file lists remaining usages (Safe Delete dialog in `crates/archipel-ide-ui/src/popups.rs`) and refuses to delete until forced; file-level text search only, not symbol-level usages (no methods, classes, parameters) |
| IJ-REF-061 | [type-migration](https://www.jetbrains.com/help/idea/type-migration.html) | Type Migration (Ctrl+Shift+F6) | Missing | 1 | — | — | not started |
| IJ-REF-062 | [encapsulate-fields](https://www.jetbrains.com/help/idea/encapsulate-fields.html) | Encapsulate Fields | Missing | 1 | — | — | not started |
| IJ-REF-063 | [convert-anonymous-to-inner](https://www.jetbrains.com/help/idea/convert-anonymous-to-inner.html) | Convert Anonymous to Inner | Missing | 1 | — | — | not started |
| IJ-REF-064 | [convert-to-instance-method](https://www.jetbrains.com/help/idea/convert-to-instance-method.html) | Convert to Instance Method | Missing | 1 | — | — | not started |
| IJ-REF-065 | [make-method-static](https://www.jetbrains.com/help/idea/make-method-static.html) | Make Static | Missing | 1 | — | — | not started |
| IJ-REF-066 | [invert-boolean-refactoring](https://www.jetbrains.com/help/idea/invert-boolean-refactoring.html) | Invert Boolean | Missing | 1 | — | — | not started |
| IJ-REF-067 | [replace-constructor-with-builder](https://www.jetbrains.com/help/idea/replace-constructor-with-builder.html) | Replace Constructor with Builder | Missing | 1 | — | — | not started |
| IJ-REF-068 | [replace-constructor-with-factory-method](https://www.jetbrains.com/help/idea/replace-constructor-with-factory-method.html) | Replace Constructor with Factory Method | Missing | 1 | — | — | not started |
| IJ-REF-069 | [replace-inheritance-with-delegation](https://www.jetbrains.com/help/idea/replace-inheritance-with-delegation.html) | Replace Inheritance with Delegation | Missing | 1 | — | — | not started |
| IJ-REF-070 | [use-interface-where-possible](https://www.jetbrains.com/help/idea/use-interface-where-possible.html) | Use Interface Where Possible | Missing | 1 | — | — | not started |
| IJ-REF-071 | [remove-middleman](https://www.jetbrains.com/help/idea/remove-middleman.html) | Remove Middleman | Missing | 1 | — | — | not started |
| IJ-REF-072 | [wrap-return-value](https://www.jetbrains.com/help/idea/wrap-return-value.html) | Wrap Method Return Value | Missing | 1 | — | — | not started |
| IJ-REF-073 | [generify-refactoring](https://www.jetbrains.com/help/idea/generify-refactoring.html) | Generify | Missing | 1 | — | — | not started |
| IJ-REF-074 | [migrate](https://www.jetbrains.com/help/idea/migrate.html) | Migrate (package/class migration maps) | Missing | 1 | — | — | not started |
| IJ-REF-075 | [find-and-replace-code-duplicates](https://www.jetbrains.com/help/idea/find-and-replace-code-duplicates.html) | Find and Replace Code Duplicates | Missing | 1 | — | — | not started |
| IJ-REF-076 | [maven-refactorings](https://www.jetbrains.com/help/idea/maven-refactorings.html) | Maven refactorings (extract managed dependency, introduce property) | Missing | 1 | — | — | not started |

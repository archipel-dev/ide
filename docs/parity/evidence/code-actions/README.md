# Quick fixes and intentions (Alt+Enter): evidence

Reference: the hover tooltip in `20-problems.png` (Android Studio: `Create function 'geet' Alt+Shift+Enter`,
`More actions... Alt+Enter`) and the IntelliJ keymap (`Show Context Actions` = Alt+Enter,
`docs/spec/keymaps.md`). There is no reference screenshot of the popup itself yet; it follows the reference
IDE's intention list (bulb icon per row, red for quick fixes of problems, yellow for intentions, quick fixes
first).

## Environment

As in `../problems/README.md` (rust-analyzer 1.97.0, jdtls 1.40.0, Xvfb, samples `rust-demo`, `java-gradle`).

## Scenes

| File | Steps | Result |
|---|---|---|
| `lightbulb-rust.png` | rust-demo `main.rs` after a save (rustc `unused_variables` warning), click into `unused` and wait | When the caret rests (350 ms) the actions are fetched in the background; a red bulb appears left of line 4 because a quick fix exists. |
| `alt-enter-rust.png` | Alt+Enter | Popup: red `if this is intentional, prefix it with an underscore: '_unused'`, `Rename unused unused to _unused`, yellow `Insert explicit type 'i32'`, `Promote local to constant`. Enter applied the first fix (`_unused`). |
| `resolved-assist-rust.png` | Alt+Enter, Down, Enter on `Insert explicit type 'i32'` | rust-analyzer defers the edit (`codeAction/resolve`); it is resolved and applied: `let _unused: i32 = 42;`, status "Applied: Insert explicit type `i32`". |
| `alt-enter-jdtls.png` | java-gradle `App.java`, caret in `greting` (line 12), Alt+Enter | jdtls quick fixes `Change to 'greeting(..)'`, `Create method 'greting(String)'`, `Add cast to 'app'` (red) and source actions (yellow). |
| `quick-fix-applied-jdtls.png` | Enter | The call becomes `app.greeting("typo")`; jdtls re-publishes diagnostics, the inspection widget turns into the green check, the red waves on the tab and tree disappear, and the bulb refreshes to yellow (only intentions left). |

Mechanics covered by tests: code actions with context diagnostics (as published), `codeAction/resolve`,
`workspace/executeCommand` and server-initiated `workspace/applyEdit` (`cargo test -p archipel-ide-lsp --test
fake_server code_actions`); atomic, validated multi-file `WorkspaceEdit` application with UTF-16 ranges,
caret tracking, version checks and one undo step per file (`cargo test -p archipel-ide-core workspace_edit`);
action parsing, disabled actions and staleness (`cargo test -p archipel-ide-core code_actions`).

## Still different from the reference

- The problem tooltip has no quick-fix row yet (`Create function … Alt+Shift+Enter`, `More actions…`), and
  there is no Alt+Shift+Enter (apply the preferred fix directly).
- No speed search, submenus (`Fix all …`, `Edit inspection profile`) or preview pane in the popup.
- Workspace edits that create, rename or delete files are refused with a message.
- Files changed by an edit that were not open are opened as (modified) background tabs; a single
  project-wide undo is still missing.

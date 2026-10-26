# Refactorings (Rename, Reformat, Optimize Imports, Extract, Inline): evidence

Reference: IntelliJ IDEA Refactor menu and editor context menu (`docs/spec/menus.md`, `RefactoringMenu`), keymap
actions `RenameElement` (Shift+F6), `ReformatCode`, `OptimizeImports`, `IntroduceVariable`, `ExtractMethod`,
`IntroduceConstant`, `Inline` (`docs/spec/keymaps.md`). Shortcuts run through the keymap engine's
`ACTION_COMMANDS`; the menus come from the reference action groups.

## Environment

Xvfb :90 1600x1000x24, `ARCHIPEL_RENDERER=glow`, `ARCHIPEL_NO_UPDATE_CHECK=1`, Eclipse JDT LS 1.40.0, OpenJDK 21,
a small Gradle Java sample (`App.java` with `greeting`, `Other.java` calling it), driven over the control socket.

## Scenes

| File | Steps | Result |
|---|---|---|
| `01-rename-preview.png` | caret in `greeting` (line 4), Shift+F6, type `salutation` | jdtls `prepareRename` opens the name field; the 3 occurrences in the file show the typed name live. |
| `02-rename-applied.png` | Enter | `workspace` edit applied to both files; status "Renamed greeting to salutation in 2 files". |
| `03-rename-other-file.png` | open `Other.java` | The background tab holds the renamed call (modified dot). |
| `04-extract-variable.png` | select `app.salutation("Archipel IDE")`, Ctrl+Alt+V | jdtls returns two variants; the intention popup lets you choose. |
| `05-extract-variable-applied.png` | Down, Enter | `String salutation = app.salutation("Archipel IDE");` is inserted and the call replaced. |
| `06-refactor-menu.png` | Refactor main menu | Reference group; Rename… and Inline… enabled (`Ctrl+Alt+N`), unimplemented entries disabled. |
| `07-context-menu-refactor.png` | right-click in the editor | Context menu with the Refactor submenu. |

Reformat Code (Ctrl+Alt+L) was run against jdtls on already formatted code ("Nothing to reformat").

Mechanics are covered by tests: fake LSP scripts `prepareRename`, `rename`, `formatting`, `rangeFormatting` and
`codeAction` with `only` (`cargo test -p archipel-ide-lsp --test fake_server`); core flows including a two-file
rename applied and undone per file (`cargo test -p archipel-ide-core refactor`).

## Still different from the reference

- The new name is typed in a popup next to the symbol, not inside the text; lines with occurrences lose syntax
  highlighting while previewing.
- No usage/conflict preview dialog, no project-wide single undo (one undo step per file), no renaming of
  comments/strings or files.
- Extract Method/Constant and Inline were verified against the fake server only (jdtls offers them as code
  actions that depend on the selection).

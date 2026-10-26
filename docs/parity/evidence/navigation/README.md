# Code navigation: evidence

Behaviour reference: IntelliJ default keymaps (`docs/spec/keymaps.md`: Go to Declaration or Usages
`Ctrl+B; Ctrl+Click`, Go to Type Declaration `Ctrl+Shift+B; Ctrl+Shift+Click`, Go to Implementation(s)
`Ctrl+Alt+B; Ctrl+Alt+Click`, Find Usages `Alt+F7`, Show Usages `Ctrl+Alt+F7`, Back/Forward
`Ctrl+Alt+Left/Right; Mouse Back/Forward`, macOS `⌘B`, `⌘[`/`⌘]`). There is no reference screenshot of the
usage views yet; the layout follows the reference IDE's Find tool window (results grouped by file, line
number, code with the usage highlighted) and its usages popup (title with count, code, `file line` at the
right).

## Environment

As in `../problems/README.md` (rust-analyzer 1.97.0, jdtls 1.40.0, Xvfb 1600x1000, samples `rust-demo` and
`java-gradle`).

## Scenes

| File | Steps | Result |
|---|---|---|
| `ctrl-hover-link.png` | rust-demo `main.rs`: hold Ctrl and hover `greet` in `greeter::greet(name)` | The identifier is underlined as a link and the pointer becomes a hand. |
| `ctrl-click-declaration-rust.png` | Ctrl+click it | `greeter.rs` opens with the caret on `greet` in `pub fn greet` (1:8). Ctrl+Alt+Left returned to `main.rs` 6:30. |
| `find-usages-rust.png` | Alt+F7 on `greet` in `main.rs` | Find tool window opens on a new `Usages of greet` tab next to `Find in Files`: "2 usages in 1 file", `main.rs src 2 usages`, rows `6 println!("{}", greeter::greet(name));` with `greet` highlighted. |
| `show-usages-on-declaration-rust.png` | In `greeter.rs` put the caret on the declaration `greet` and press Ctrl+B | Like the reference IDE, Go to Declaration on a declaration shows its usages: popup "Usages of greet 3 results" with the declaration and both calls, `file line` on the right. Down, Enter jumped to `main.rs` 6:29. |
| `find-usages-jdtls.png` | java-gradle `App.java`: caret in `app.greeting(name)` (line 14), Alt+F7 | jdtls usages: `Usages of greeting`, 1 usage in `App.java src/main/java/com/example` line 14. |
| `ctrl-click-declaration-jdtls.png` | Ctrl+click `greeting` on line 14 | Caret moves to the method declaration at 6:19. |

## Still different from the reference

- The reference colours the Ctrl+hover link text blue and shows a quick documentation tooltip; Archipel IDE
  only underlines.
- The usages tab has no toolbar (rerun, group by, preview pane), no read/write usage split and no
  module/package grouping; only one usages tab is kept.
- Declarations outside the project (JDK, Rust std sources) cannot be opened yet: the status bar says so.
- No "Find Usages in File" (`Ctrl+F7`), Quick Definition or Highlight Usages in File yet.

## Reproduce

```bash
cargo test -p archipel-ide-lsp --test fake_server navigation
cargo test -p archipel-ide-lsp locations
cargo test -p archipel-ide-core navigation
```

# Problems and error highlighting: evidence

Reference scene: `20-problems.png` from IntelliJ IDEA Community (java-gradle sample, `app.greting("typo");`
after line 11) and Android Studio (`docs/parity/evidence/reference/*/20-problems.png`, notes in each
`NOTES.md`).

## Environment

- Linux, Xvfb 1600x1000, `ARCHIPEL_RENDERER=glow ARCHIPEL_NO_UPDATE_CHECK=1`, xdotool.
- rust-analyzer 1.97.0 (rustup component), Eclipse JDT LS 1.40.0 (`jdtls` on PATH), OpenJDK 21, Gradle.
- `/home/user/samples/java-gradle`: `settings.gradle.kts`, `build.gradle.kts` (application plugin) and
  `src/main/java/com/example/App.java` as in the reference scene, with `app.greting("typo");` on line 12.
- `/home/user/samples/rust-demo`: a Cargo binary whose `main.rs` calls `greeter::geet(name)` (unresolved),
  has an unused `let unused = 42;`, and whose `greeter.rs` contains `let x: i32 = "loud";`.

## Scenes

| File | Steps | Result |
|---|---|---|
| `20-problems-jdtls.png` | Open java-gradle with `App.java`, wait for the jdtls import, expand `src/main/java/com/example` in the Project tree, click the `❗1` counter of the inspection widget, hover `greting` | Matches the reference layout: red squiggle under `greting`, inspection widget `❗1 ˄ ˅` top right, red error-stripe mark level with line 12, red wave under the tab name and under `java-gradle`, `src`, `main`, `java`, `com`, `example`, `App.java`; the counter click opened the Problems tool window with tabs `File 1` / `Project Errors 1`, row `App.java /home/user/…/com/example 1 problem` > `❗ The method greting(String) is undefined for the type App :12`; tooltip with the message and `Java(67108964)`. |
| `rust-analyzer-error-file-tab.png` | Open rust-demo with `main.rs`, type and delete a space, Ctrl+S, open Problems | Saving sends `textDocument/didSave`, rust-analyzer runs `cargo check`; `rustc(E0425)` "cannot find function `geet`" with squiggle, stripe mark, counter and the File tab. |
| `rust-analyzer-project-errors-tab.png` | Same, click `Project Errors` | Errors of the never-opened `greeter.rs` (from `cargo check`) are listed next to `main.rs`, grouped by file. |
| `rust-analyzer-unused-warning.png` | F2 (caret jumps to `geet`, status shows the message), → , type `r`, Ctrl+S, hover `unused` | The error is gone; the counter switches to `⚠1`; `unused` is greyed out (unnecessary tag) and the tooltip shows the rustc warning with `rustc(unused_variables)`. |

F2 / Shift+F2 move between problems (errors first, wrapping); Alt+6 toggles the tool window; clicking a row
opens the file and puts the caret on the problem; clicking a stripe mark jumps to it.

## Still different from the reference

- IntelliJ colours unresolved references red (`WRONG_REFERENCES_ATTRIBUTES`) instead of a wave; Archipel IDE
  draws a wave for every error because LSP does not distinguish unresolved references.
- No quick-fix row (`Create function … Alt+Shift+Enter`, `More actions… Alt+Enter`) in the tooltip yet:
  code actions are the next slice.
- The Problems tool window has no left toolbar (preview, quick fix, grouping options), no collapsible
  file nodes and no Vulnerable Dependencies / Qodana tabs. Project Errors lists errors only, like the
  reference; it only knows files the servers published diagnostics for (rust-analyzer: `cargo check`
  results; jdtls: the files of the imported project it has built).
- Columns of files that are not open are the server's UTF-16 columns.
- No gutter icons (the reference shows none for problems either).

## Reproduce

```bash
cargo test -p archipel-ide-lsp --test fake_server diagnostics
cargo test -p archipel-ide-core problems
cargo test -p archipel-ide-ui problems_ui
```

# Roadmap

The Android Studio parity programme (next PRs, in order) is in
[roadmap-as-parity.md](roadmap-as-parity.md).

## M0 — repository and contracts (complete)

- Modular workspace and enforced dependency direction by convention.
- Typed command registry shared by native UI and CLI.
- Honest parity inventory and generated summary command.
- Host/Android Studio inventory documented.

## M1 — usable native shell (in progress)

- [x] Native window and dark IDE layout
- [x] Open project from startup path
- [x] Bounded project tree
- [x] Real editable files in tabs
- [x] Save and Save All commands
- [x] Command palette
- [x] Manifest plugin discovery
- [x] Plugin API 1: Rust plugins contribute commands, menus, keybindings, settings, tool windows, editor tabs and status widgets; live enable/disable; `--app <id>` standalone mode (`docs/plugins/api.md`)
- [x] `.jtplugin` install/uninstall without restart and Plugins page (Marketplace/Installed tabs)
- [x] Lua 5.4 plugins (sandboxed; immediate and declarative views)
- [x] WASM component plugins (wasmtime, WIT world, permission-gated WASI, fuel/memory limits)
- [ ] JetBrains Marketplace browsing and conversion
- [x] Revisioned rope-backed text engine with Unicode-safe undo/redo
- [x] Viewport-native editor that shapes and paints only visible lines
- [x] Multi-cursor selections, atomic edits, and coalesced primary typing transactions
- [x] Tree-sitter Rust parsing, syntax highlighting, and parse diagnostics
- [x] Data-driven Tree-sitter language registry (20 grammars) with bundled highlight queries
- [x] IntelliJ editor color schemes (Islands Dark) resolved by text-attribute key
- [ ] Incremental Tree-sitter edit notifications, viewport-bounded highlighting, injections
- [ ] Tree-sitter folding, Expand/Shrink Selection, Structure view and File Structure popup
- [x] Integrated native PTY terminal with shell input and command dispatch
- [x] Interactive ANSI terminal grid, direct shell keys, selection/copy, scrollback and resize
- [ ] Terminal hyperlinks, mouse reporting and multiple sessions
- [x] Cancellable background project text search with streamed result navigation
- [x] Detected Cargo/Gradle/Maven/npm/Go builds with streamed output and cancellation
- [x] UI automation control socket (`JETTHING_CONTROL_SOCKET`, `jetthing-cli ui`)
- [x] Scripted parity scenes (`xtask scenes`: 01, 04, 05, 06, 11, 12) and headless screenshot CI job
- [x] New UI "Islands Dark" workbench: merged toolbar, stripe, islands, breadcrumbs status bar
- [x] One-tool-window-per-anchor model with typed `toolwindow.*` commands
- [x] Search Everywhere (Double Shift) over actions and files with fuzzy ranking
- [x] Collapsible, sorted project tree with VCS file colors
- [x] Run widget (`run.project`) next to Build, with OpenGL fallback renderer
- [x] Release packaging (`cargo xtask dist`) and CI/release workflows
- [x] Typed settings model and Settings dialog with search, Apply/Reset and Advanced Settings
- [x] Theme engine from spec data with live Islands Dark / Islands Light switching
- [x] Keymap engine loading all bundled keymaps, keymap-driven shortcuts and Keymap settings page
- [ ] Bundled icon set beyond the core actions
- [x] Native installers: MSI, DMG (`Jetthing.app`) and `.deb`, smoke-tested in CI
- [ ] Code signing and notarization with project certificates (pipeline supports it)
- [x] Editor actions: duplicate/delete/comment/move lines, pairing, smart Enter, word moves
- [x] Find/Replace in file, Go to Line, Recent Files, Go to File
- [x] Project tree New/Rename/Delete, commit with message and file selection
- [x] Workspace restore (`.jetthing/workspace.json`), autosave on focus loss
- [x] Check for Updates with verified self-update from GitHub releases
- [x] `--version` and Help → About with embedded build info (commit, date, target)
- [x] Log file per OS, crash reports with backtrace, crash notice on next start
- [x] Release notes from conventional commits (`cargo xtask changelog`)
- [x] First-run "Customize Jetthing": theme, keymap, settings import
- [x] Versioned release pipeline (0.0.2) publishing on every version bump

## AI Assistant (#52)

- [x] `jetthing-ai` engine: provider trait, Anthropic/OpenAI-compatible/Ollama streaming, mock provider, settings and key storage
- [x] AI Chat tool window with streaming Markdown, context chips, slash commands, Apply with diff preview
- [x] Inline ghost-text completion with debounce, type-through, Tab / word accept
- [x] AI actions (Explain, Refactor, Generate tests/docs, Find problems, Fix with AI on problems and failed builds)
- [x] Commit message generation from the selected changes
- [x] Terminal natural language → command with confirmation
- [x] Agent mode with a visible plan, per-edit/command approval and Stop

## M2 — language intelligence

VFS snapshots, background indexing, LSP transport, diagnostics, completion, symbol
navigation, formatting, cancellation, and persistent caches. First verified integration:
Rust via rust-analyzer.

- [x] JSON-RPC/LSP process transport and initialization
- [x] Multi-language server registry (provider trait, PATH/`lsp.toml` discovery), lazy start per
      workspace root, crash restart with backoff, `$/progress` status widget, missing-server notices
- [x] Rust document open/change synchronization and published diagnostics
- [x] Completion request transport and result modeling
- [x] Completion popup, UTF-16 caret integration, and transactional insertion
- [x] IntelliJ-style completion popup: auto-popup, camel-hump filtering/ranking, Enter/Tab
- [x] Completion snippets with tab stops, mirrors and auto-import edits
- [x] Completion documentation panel (resolve), quick documentation and hover
- [x] Parameter info (signature help) after `(`/`,`, after completing calls, and Ctrl+P
- [ ] Navigation, formatting, code actions

## M3 — professional workflows

Git, search/replace, build/run configurations, test UI, DAP debugger, terminal,
workspace layout persistence, plugin manager, and recovery.

- [x] Repository detection, branch/status view, stage, and unstage
- [ ] Diff editor, history, blame, commit, branches, fetch/pull/push, and worktrees
- [x] Universal process runner and detected project build command
- [x] Build-system provider trait; Build/Run window with step tree, timings and error navigation
- [x] Gradle sync model (init script) and Gradle tool window
- [x] Persistent run configurations with the Run/Debug Configurations dialog
- [x] Test runner tree with Rerun Failed Tests (JUnit XML, libtest, go test)
- [ ] Debug-before-run and compound tasks

## M4 — web/JVM/Android bundles

Product-neutral bundles start with Web and Rust, then Java/Kotlin/Gradle and Android.
Use installed SDK, ADB, emulator, and Gradle; do not depend on Android Studio.

- [x] Toolchain provisioning core (`jetthing-toolchains`): verified resumable downloads, atomic
      installs, PATH-first discovery, project pins, `toolchain.*` commands, status-bar progress
- [ ] "Missing toolchain" editor notification with Set up, Settings > Toolchains page
- [ ] Catalog: JDK (foojay Disco) + jdtls, rust-analyzer/rustup, Android cmdline-tools/sdkmanager,
      kotlin-lsp, bazelisk/starpls, Node/typescript-language-server, pyright/ruff, gopls/delve,
      clangd, lemminx, taplo, yaml-language-server
- [x] LSP "server not found" → Set up through the resolver (jdtls on a managed JDK, rust-analyzer, taplo)
- [ ] Build/run, Android, Bazel and debuggers resolve their tools through `Toolchains`

## Next task

Add incremental Tree-sitter edit notifications and expand the editor interaction model
with clipboard operations, find/replace, and IME input (vertical movement, smart Home/End
and auto-indent on Enter landed with the workbench restyle). Add a UI control socket so
the screenshot walkthrough in `docs/parity/evidence/` becomes an automated test.

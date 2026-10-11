# Architecture

Jetthing is a native Rust workspace with a headless engine and replaceable UI shell.
The initial shell uses eframe/egui with wgpu. This choice provides a portable native
window, GPU rendering, IME/input handling, accessibility work in progress upstream,
and fast delivery of the first usable slice. It is not an architectural dependency:
no crate below `jetthing-ui` imports UI types.

> **Archipel SDK.** `jetthing-command`, `jetthing-plugin-api` and `jetthing-plugin-ui` are thin
> re-export shims over `archipel-command`, `archipel-plugin-api` and `archipel-plugin-ui` from the
> [`archipel-dev/sdk`](https://github.com/archipel-dev/sdk) repository (git dependency on `main`).
> The SDK is the source of truth for these APIs; change them there, not here. The shims keep the
> old crate names so that dependents compile unchanged.

## Layers

- `jetthing-command`: typed command metadata, JSON parameters/results, discovery,
  validation, execution, and structured errors.
- `jetthing-project`: safe project-root model, bounded file tree, UTF-8 document IO.
- `jetthing-text`: revisioned rope storage, Unicode-safe multi-selection transactions,
  typing coalescence, and history.
- `jetthing-language`: data-driven Tree-sitter language registry (grammar crates plus their
  bundled queries), a generic highlight engine producing `HighlightLayer`s keyed by IntelliJ
  text-attribute keys, and the editor color-scheme model loaded from `docs/spec/data/`.
- `jetthing-terminal`: native PTY process lifecycle, asynchronous output, input, and resizing.
- `jetthing-search`: cancellable generation-based background text search with streamed,
  bounded results and binary/large-file exclusion.
- `jetthing-git`: repository/branch/status modeling and argument-safe Git operations;
  it delegates repository semantics to the installed Git executable.
- `jetthing-runner`: the `BuildSystem` provider trait and registry (Cargo, Maven, Bazel,
  npm, Go built in), cancellable child-process execution, a line-oriented console model
  (`\r` progress, ANSI stripping) and a build-event parser (steps with durations, compiler
  messages with source locations).
- `jetthing-runnables`: entry points (mains, tests, test containers) found with the
  registry's tree-sitter grammars, for gutter run icons.
- `jetthing-bazel`: Bazel workspace detection, `bazel query` target model with a cache,
  BUILD-file rule scanning, and per-target run configurations.
- `jetthing-gradle`: static Gradle build detection (settings includes, wrapper, plugin
  ids/aliases including Android) and the Gradle `BuildSystem` provider.
- `jetthing-android`: Android SDK discovery (IDE setting, `local.properties`, `ANDROID_HOME`,
  `ANDROID_SDK_ROOT`, OS default), installed-package inventory, and Android Gradle project
  detection; host access goes through the `HostEnvironment` trait so tests use fakes.
- `jetthing-lsp`: JSON-RPC framing (`transport`), one server process with typed
  request/response plumbing, cancellation, `$/progress` and diagnostics (`client`), the
  server registry behind the `LanguageServerProvider` trait with PATH/`lsp.toml` discovery
  (`registry`), and `LspManager`, which starts servers lazily per workspace root and
  restarts crashed ones (`manager`). Uses `lsp-types`; no UI code. Popups live in
  `jetthing-ui`. `jetthing-fake-lsp` is a scriptable test server.
- `jetthing-toolchains`: toolchain provisioning (language servers, JDKs/SDKs, build tools,
  debuggers, formatters). Component model and `ComponentProvider` catalog trait, version
  requests, resumable SHA-256-verified downloads through `HTTPS_PROXY`/`NO_PROXY`, safe
  `tar.gz`/`tar.xz`/`zip`/`gz` extraction, atomic installs under
  `<data dir>/jetthing/toolchains/<component>/<version>/`, PATH-first discovery, project pins
  and a background install service. No UI code; see "Toolchain provisioning" below.
- `jetthing-plugin-api`: the toolkit-free plugin contract (`Plugin`, contributions,
  `PluginHost`, `jetthing-plugin.toml` manifest, opaque `ViewFrame`); see `docs/plugins/api.md`.
- `jetthing-plugins`: plugin registry: discovery, activation order, enable/disable and
  settings persistence. Lua/Wasm loading is deliberately not claimed yet.
- `jetthing-plugin-ui`: egui helpers for plugin views (UI layer, used by plugin crates).
- `jetthing-keymap`: keystroke/chord/mouse/gesture parsing, the 26 bundled keymaps from
  `docs/spec/data/keymaps.json`, desktop default selection, user overrides, lookup and
  conflict detection.
- `jetthing-actions`: catalog of the reference action ids, texts and groups from
  `docs/spec/data/actions.json` (names for the keymap page, structure for menus).
- `jetthing-settings`: typed settings registry (bool/int/float/text/path/enum/list with
  defaults and validation), the Settings tree keyed by the reference configurable ids in
  `docs/spec/data/settings.json`, application/project layering, TOML persistence,
  search, and the dialog's draft/Apply/Reset model. No UI types.
- `jetthing-plugin-ui`: egui helpers for plugin views and the declarative `ViewNode` renderer
  (UI layer, used by plugin crates and script runtimes).
- `jetthing-plugin-lua`: sandboxed Lua 5.4 plugin runtime (`LuaLoader`), registered by the app.
- `jetthing-plugin-wasm`: WebAssembly component plugin runtime (`WasmLoader`, wasmtime + WIT) with
  permission-gated WASI, registered by the app.
- `jetthing-core`: application state, the plugin host (scoped `PluginHost`, command
  routing, plugin tool window/tab state), tool window layout model (one visible tool window
  per anchor), the problems model (`problems`: syntax errors and LSP diagnostics merged
  into per-file problems with severities, counts and ancestor error propagation), and
  built-in command handlers.
- `jetthing-ui`: native workbench shell split into `chrome` (main toolbar, stripe, status
  bar, window controls), `toolwindows` (Project, Commit, Find, Build/Run, Problems,
  Terminal), `editor` (tabs, viewport-native highlighted editor, completion popup),
  `palette` (Search Everywhere), `theme` / `theme_engine` (color tokens resolved from
  `docs/spec/data` for the active theme, fonts, egui visuals), and `icons`
  (original 16×16 vector icons).
- `jetthing-app`: process startup, native window (custom title bar on Windows/Linux),
  and wgpu → OpenGL renderer fallback.
- `jetthing-control`: UI automation wire protocol (JSON lines), local-socket listener and
  blocking client; toolkit-independent. The workbench drains requests each frame.
- `jetthing-cli`: headless command discovery and execution, and `ui …` requests to a
  running workbench over the control socket.

## Toolchain provisioning

`jetthing-toolchains` is the single place that finds or installs the external programs a
language needs. Other subsystems (LSP registry, build/run providers, Android, Bazel,
debuggers) do not search `PATH` themselves; they ask the resolver:

```rust
let toolchains: &Toolchains = &state.toolchains.toolchains; // jetthing-core
toolchains.executable("jdtls");            // Option<PathBuf>
toolchains.locate("jdk");                  // Option<Located { executable, home, version, source }>
toolchains.missing(&["jdtls"]);            // ["jdtls", "jdk"] incl. `requires`, when not found
toolchains.environment(&["jdk", "jdtls"]); // PATH prefix + JAVA_HOME for child processes
```

Lookup order: `path` configured in `toolchains.toml` → managed install matching the project
pin (when a version is pinned) → home variable (`JAVA_HOME`, `ANDROID_HOME`) → `PATH` → newest
managed install. Pins come from `.jetthing/toolchains.toml` first, then `rust-toolchain(.toml)`,
`.bazelversion`, Gradle `JavaLanguageVersion.of(n)`/`jvmToolchain(n)`,
`gradle/gradle-daemon-jvm.properties`, `.sdkmanrc`, `.nvmrc`/`.node-version`,
`.python-version` and `go.mod`.

Language bundles describe components with a `ComponentSpec` (id, executables, bin dirs, home
variables, required components) and resolve versions to a `Release` (URL, SHA-256 or checksum
URL, archive kind) by implementing `ComponentProvider`; `StaticProvider` and `IndexProvider`
(JSON index, e.g. a corporate mirror) cover simple sources. Built-in providers live in
`jetthing_toolchains::providers`: `jdk` (foojay Disco API, Temurin unless `distribution` is
set), `jdtls` (Eclipse milestones), and GitHub-release tools (`rust-analyzer`, `bazelisk`,
`taplo`) verified with the asset `digest` or a published checksum file. Nothing downloads without consent:
installs start only from the `toolchain.install` command. Downloads resume with `Range`
requests, are verified before extraction (a mismatch deletes the file), extract into
`.staging/` and are renamed into place, so a version directory is either complete or absent.

Language servers reach managed installs through `jetthing_lsp::ProgramResolver`, a hook the
`LspManager` consults after an `lsp.toml` command and before `PATH` candidates;
`jetthing-core` implements it over `Toolchains` (`toolchain_lsp.rs`, e.g. jdtls runs on the
resolved JDK) and attaches the providing component to missing-server notices, whose **Set up**
action runs `toolchain.setup` and restarts the servers when it finishes.

Commands: `toolchain.list`, `toolchain.locate`, `toolchain.missing`, `toolchain.setup`
(component plus missing requirements), `toolchain.install`
(`wait: true` blocks, for the CLI), `toolchain.cancel`, `toolchain.jobs`, `toolchain.uninstall`,
`toolchain.usePath` ("use existing installation", stored in the user `toolchains.toml`) and
`toolchain.pins`. Running installs show in the status bar with a progress bar; clicking it cancels.
`JETTHING_TOOLCHAINS_DIR` overrides the install root.

## Command contract

Commands have stable IDs, human labels, JSON schemas expressed as documentation,
capability requirements, an enabled predicate, and an execution handler. GUI and CLI
use the same registry, and the control socket's `execute` request reaches every command of
a running workbench. Cancellation/progress and MCP transports are next-step extensions of
this contract.

## Keymap

`IdeState::keymap` holds the effective keymap (Settings › Keymap plus the user's
overrides in `keymap.overrides`). The UI turns key presses into keystrokes, resolves them
with `Keymap::lookup` (two-stroke chords included) and runs the first applicable action:
a command from `jetthing_core::keymap_commands::ACTION_COMMANDS` (IntelliJ action id →
command id, global or editor scope) or a workbench popup (`jetthing-ui::keybindings`).
To make a new command reachable by its reference shortcut, add its IntelliJ action id to
`ACTION_COMMANDS`; menus and Find Action then show the keymap's shortcut for it.

## Settings extension point

Settings pages are data, not UI code. A subsystem that needs a page registers it with
the registry held in `IdeState::settings` (normally from its `jetthing-core` command
module, before the UI starts):

```rust
use jetthing_settings::{PageDef, Scope, SettingDef, SettingKind, SettingValue};

let registry = &mut state.settings.registry;
// Reuse the reference page id when the page exists in docs/spec/data/settings.json
// (e.g. `reference.settingsdialog.project.gradle`); register a new page only otherwise.
registry.register_page(PageDef::new("jetthing.rust", Some("language"), "Rust"))?;
registry.register_setting(
    SettingDef::new("rust.analyzerPath", "jetthing.rust", "rust-analyzer executable",
        SettingKind::Path, SettingValue::Text(String::new()))
        .group("Language Server")
        .scope(Scope::Project)
        .description("Empty searches PATH"),
)?;
// Read the effective value wherever it is used:
let path = state.settings.text("rust.analyzerPath");
```

The Settings dialog renders registered settings generically (checkbox, combo box,
number and text fields, grouped under headings), includes them in search, marks
modified pages and applies edits through `settings.apply`. Values are stored in
`<config dir>/settings.toml` (application) or `<project>/.jetthing/settings.toml`
(project); the config dir is `JETTHING_CONFIG_DIR` or the platform default. Pages that
need bespoke UI (Keymap, Plugins) get a page renderer in `jetthing-ui::settings_dialog`.
Commands: `settings.open {page?}`, `settings.get/set/reset {id}`,
`settings.apply {changes}`, `settings.search {query}`.

## Long-term direction

The semantic platform will layer VFS snapshots, incremental syntax, symbol/stub
indexes, dependency-aware invalidation, and prioritized cancellable jobs. LSP is one
provider behind a richer language-service interface, not the semantic architecture.
Untrusted compiled plugins use the WebAssembly Component Model with explicit capabilities
(`jetthing-plugin-wasm`); native plugins remain compiled-in and trusted.

## Reference findings

PhotoCraft validates the engine-first command-registry approach and generated parity
reports. IntelliJ's VFS/PSI/stub indexes motivate stable virtual identities and compact
externally-visible symbol indexes. Zed demonstrates GPU-first native UI; Lapce validates
rope-based text storage and language-service isolation. Jetthing reimplements behavior
cleanly and does not reuse protected assets or branding.

# Architecture

Archipel IDE is a native Rust workspace with a headless engine and replaceable UI shell.
The initial shell uses eframe/egui with wgpu. This choice provides a portable native
window, GPU rendering, IME/input handling, accessibility work in progress upstream,
and fast delivery of the first usable slice. It is not an architectural dependency:
no crate below `archipel-ide-ui` imports UI types.

> **Archipel SDK.** `archipel-ide-command`, `archipel-ide-plugin-api` and `archipel-ide-plugin-ui` are thin
> re-export shims over `archipel-command`, `archipel-plugin-api` and `archipel-plugin-ui` from the
> [`archipel-dev/sdk`](https://github.com/archipel-dev/sdk) repository (git dependency on `main`).
> The SDK is the source of truth for these APIs; change them there, not here. The shims keep the
> old crate names so that dependents compile unchanged.

## Naming scheme

The product is **Archipel IDE**. It is one tool of the Archipel suite, so names that are
shared with the suite stay short and names that belong to the IDE say so:

| Kind | Name | Notes |
|---|---|---|
| Crates (packages, directories, lib names) | `archipel-ide-<name>` / `archipel_ide_<name>` | **every** IDE crate, e.g. `archipel-ide-core`, `archipel-ide-command`, `archipel-ide-plugin-api` |
| Binaries | `archipel-ide` (`archipel-ide-app`) and `archipel-cli` (`archipel-ide-cli`) | `[[bin]]` names; installers ship these names |
| Environment variables | `ARCHIPEL_*` | |
| Per-project data | `.archipel/` | |
| Per-user config, data, cache, logs | `archipel` (`Archipel` on macOS and Windows) | e.g. `~/.config/archipel` |
| Bundle and package ids | `dev.archipel.ide` (macOS bundle), `archipel-ide` (deb, Homebrew, Scoop), `ArchipelDev.ArchipelIDE` (winget) | |
| Bundled plugin ids | `dev.archipel.<name>` | |
| WIT package | `archipel:plugin@1.0.0` | |

Why the `archipel-ide-` prefix on all crates rather than only on the ones that collide: the
workspace depends on the suite crates `archipel-kit`, `archipel-icons` (kit repository) and
`archipel-command`, `archipel-plugin-api`, `archipel-plugin-testing`, `archipel-plugin-ui`
(SDK repository). Three IDE crates would have received the same package names as SDK crates
(`command`, `plugin-api`, `plugin-ui`; they are the re-export shims above), and Cargo allows
only one package per name in a dependency graph. A mixed scheme (`archipel-ide-command` next
to `archipel-core`) would make it impossible to tell from a crate name which repository owns
it, so the prefix is applied to all of them: `archipel-<x>` is a suite crate from another
repository, `archipel-ide-<x>` belongs to this workspace.

### Compatibility with the Jetthing names

The IDE was called Jetthing before this rename. User-facing persisted data and automation
keep working; the old names are known in exactly one place, `archipel-ide-compat` (no
dependencies, below every other crate), plus a few documented spots next to the data format
they concern. Each fallback logs a one-time warning (stderr and the log file).

| Old | New | Behaviour |
|---|---|---|
| `JETTHING_*` environment variables | `ARCHIPEL_*` | the new name wins; the old name is read as a fallback (`archipel_ide_compat::env_var[_os]`); `build.rs` reads `JETTHING_BUILD_COMMIT` too |
| `<project>/.jetthing/` | `<project>/.archipel/` | read from the old directory while the new one does not exist; the first write copies the whole old tree to `.archipel/` and continues there; the old directory is left in place; both are hidden from the Project tree, the file watcher and VCS noise |
| per-user config, data (toolchains), cache and log directories `jetthing` / `Jetthing` | `archipel` / `Archipel` | moved to the new name the first time the new path is resolved (rename; copy if rename fails; the old directory is used if both fail). Toolchain receipts `.jetthing-install.json` are still read |
| mongo-raabit `connections.json` | same path under `archipel/` | copied from `jetthing/mongo-raabit/` on first use |
| bundled plugin ids `dev.jetthing.*` | `dev.archipel.*` | `plugins.json` (disabled flags, plugin settings) and `plugin-data/<id>` are migrated on load |
| `jetthing-plugin.toml` | `archipel-plugin.toml` | both are read (handled by the SDK's `MANIFEST_FILES`) |
| `.jtplugin` archives | `.archipelplugin` | both are installed and listed; `pack` writes `.archipelplugin` |
| WIT package `jetthing:plugin@1.0.0` | `archipel:plugin@1.0.0` | components built for either are loaded: the loader binds both worlds (`wit/` and `wit-legacy/`) and picks the one the component imports; see `docs/plugins/wasm.md` |
| Lua global `jetthing` | `archipel` | `jetthing` stays as an alias of the same table |
| setting value `gradle.buildRunUsing = "jetthing"` | `"archipel-ide"` | translated when the settings file is read |
| control socket `JETTHING_CONTROL_SOCKET` | `ARCHIPEL_CONTROL_SOCKET` | via the environment fallback; `archipel-cli ui` reads the same variable |

Not migrated: release assets and in-app updates (installed Jetthing builds look for
`jetthing-*` assets in the old repository; the first Archipel IDE build must be installed
manually), the MSI install folder (the upgrade code is unchanged, so the MSI upgrade removes
the old `Jetthing` folder), and the screenshots under `docs/parity/evidence/`, which still
show the old product name.

## Layers

- `archipel-ide-compat`: the old Jetthing names (environment variables, `.jetthing/`,
  per-user directories) and how they map to the current ones; no dependencies, see
  "Compatibility with the Jetthing names".
- `archipel-ide-command`: typed command metadata, JSON parameters/results, discovery,
  validation, execution, and structured errors.
- `archipel-ide-project`: safe project-root model, bounded file tree, UTF-8 document IO.
- `archipel-ide-text`: revisioned rope storage, Unicode-safe multi-selection transactions,
  typing coalescence, and history.
- `archipel-ide-language`: data-driven Tree-sitter language registry (`LanguageDef`: grammar
  crates plus their bundled queries, token sets, completeness tier, services hook), a generic
  highlight engine producing `HighlightLayer`s keyed by IntelliJ text-attribute keys, the syntax
  layer (`SyntaxSnapshot`/`SyntaxNode`, element types, `Anchor`, injected fragments), the
  language-keyed `ServiceRegistry`, and the editor color-scheme model loaded from `docs/spec/data/`.
- `archipel-ide-analysis`: the native code-analysis pipeline core: `FileId`/`FileTable`, VFS
  facade and document overlay, `AnalysisHost`/`Snapshot`/`Change`, background jobs with
  cancellation, modification stamps and cached values with early cutoff, and the extension
  contracts (index, daemon, inspections, intentions, structure, resolve, completion, refactoring,
  LSP provider) with in-memory implementations. No UI code; see
  [analysis/architecture.md](analysis/architecture.md).
- `archipel-ide-terminal`: native PTY process lifecycle, asynchronous output, input, and resizing.
- `archipel-ide-search`: cancellable generation-based background text search with streamed,
  bounded results and binary/large-file exclusion.
- `archipel-ide-git`: repository/branch/status modeling and argument-safe Git operations;
  it delegates repository semantics to the installed Git executable.
- `archipel-ide-runner`: the `BuildSystem` provider trait and registry (Cargo, Maven, Bazel,
  npm, Go built in), cancellable child-process execution, a line-oriented console model
  (`\r` progress, ANSI stripping) and a build-event parser (steps with durations, compiler
  messages with source locations).
- `archipel-ide-runnables`: entry points (mains, tests, test containers) found with the
  registry's tree-sitter grammars, for gutter run icons.
- `archipel-ide-bazel`: Bazel workspace detection, `bazel query` target model with a cache,
  BUILD-file rule scanning, and per-target run configurations.
- `archipel-ide-gradle`: static Gradle build detection (settings includes, wrapper, plugin
  ids/aliases including Android) and the Gradle `BuildSystem` provider.
- `archipel-ide-projectmodel`: the project model (modules, content and source roots, SDKs,
  libraries, order entries with dependency scopes), `ProjectFileIndex` and `GlobalSearchScope`s;
  pure data. `archipel-ide-externalsystem`: the Gradle, Maven, Cargo and Bazel importers that
  produce it (`DataNode` trees, data services, auto-import support). See
  `docs/architecture/project-model.md`.
- `archipel-ide-android`: Android SDK discovery (IDE setting, `local.properties`, `ANDROID_HOME`,
  `ANDROID_SDK_ROOT`, OS default), installed-package inventory, and Android Gradle project
  detection; host access goes through the `HostEnvironment` trait so tests use fakes.
- `archipel-ide-lsp`: JSON-RPC framing (`transport`), one server process with typed
  request/response plumbing, cancellation, `$/progress` and diagnostics (`client`), the
  server registry behind the `LanguageServerProvider` trait with PATH/`lsp.toml` discovery
  (`registry`), and `LspManager`, which starts servers lazily per workspace root and
  restarts crashed ones (`manager`). Uses `lsp-types`; no UI code. Popups live in
  `archipel-ide-ui`. `archipel-fake-lsp` is a scriptable test server.
- `archipel-ide-toolchains`: toolchain provisioning (language servers, JDKs/SDKs, build tools,
  debuggers, formatters). Component model and `ComponentProvider` catalog trait, version
  requests, resumable SHA-256-verified downloads through `HTTPS_PROXY`/`NO_PROXY`, safe
  `tar.gz`/`tar.xz`/`zip`/`gz` extraction, atomic installs under
  `<data dir>/archipel/toolchains/<component>/<version>/`, PATH-first discovery, project pins
  and a background install service. No UI code; see "Toolchain provisioning" below.
- `archipel-ide-plugin-api`: the toolkit-free plugin contract (`Plugin`, contributions,
  `PluginHost`, `archipel-plugin.toml` manifest, opaque `ViewFrame`); see `docs/plugins/api.md`.
- `archipel-ide-plugins`: plugin registry: discovery, activation order, enable/disable and
  settings persistence. Lua/Wasm loading is deliberately not claimed yet.
- `archipel-ide-plugin-ui`: egui helpers for plugin views (UI layer, used by plugin crates).
- `archipel-ide-keymap`: keystroke/chord/mouse/gesture parsing, the 26 bundled keymaps from
  `docs/spec/data/keymaps.json`, desktop default selection, user overrides, lookup and
  conflict detection.
- `archipel-ide-actions`: catalog of the reference action ids, texts and groups from
  `docs/spec/data/actions.json` (names for the keymap page, structure for menus).
- `archipel-ide-settings`: typed settings registry (bool/int/float/text/path/enum/list with
  defaults and validation), the Settings tree keyed by the reference configurable ids in
  `docs/spec/data/settings.json`, application/project layering, TOML persistence,
  search, and the dialog's draft/Apply/Reset model. No UI types.
- `archipel-ide-plugin-ui`: egui helpers for plugin views and the declarative `ViewNode` renderer
  (UI layer, used by plugin crates and script runtimes).
- `archipel-ide-plugin-lua`: sandboxed Lua 5.4 plugin runtime (`LuaLoader`), registered by the app.
- `archipel-ide-plugin-wasm`: WebAssembly component plugin runtime (`WasmLoader`, wasmtime + WIT) with
  permission-gated WASI, registered by the app.
- `archipel-ide-core`: application state, the plugin host (scoped `PluginHost`, command
  routing, plugin tool window/tab state), tool window layout model (one visible tool window
  per anchor), the problems model (`problems`: syntax errors and LSP diagnostics merged
  into per-file problems with severities, counts and ancestor error propagation), the analysis
  seam (`analysis_host`: `IdeState::analysis` kept in sync with the open documents by
  `poll_analysis`; `analysis_modules`: one registration line per analysis package), and
  built-in command handlers.
- `archipel-ide-ui`: native workbench shell split into `chrome` (main toolbar, stripe, status
  bar, window controls), `toolwindows` (Project, Commit, Find, Build/Run, Problems,
  Terminal), `editor` (tabs, viewport-native highlighted editor, completion popup),
  `palette` (Search Everywhere), `theme` / `theme_engine` (color tokens resolved from
  `docs/spec/data` for the active theme, fonts, egui visuals), and `icons`
  (original 16×16 vector icons).
- `archipel-ide`: process startup, native window (custom title bar on Windows/Linux),
  and wgpu → OpenGL renderer fallback.
- `archipel-ide-control`: UI automation wire protocol (JSON lines), local-socket listener and
  blocking client; toolkit-independent. The workbench drains requests each frame.
- `archipel-cli`: headless command discovery and execution, and `ui …` requests to a
  running workbench over the control socket.

## Toolchain provisioning

`archipel-ide-toolchains` is the single place that finds or installs the external programs a
language needs. Other subsystems (LSP registry, build/run providers, Android, Bazel,
debuggers) do not search `PATH` themselves; they ask the resolver:

```rust
let toolchains: &Toolchains = &state.toolchains.toolchains; // archipel-ide-core
toolchains.executable("jdtls");            // Option<PathBuf>
toolchains.locate("jdk");                  // Option<Located { executable, home, version, source }>
toolchains.missing(&["jdtls"]);            // ["jdtls", "jdk"] incl. `requires`, when not found
toolchains.environment(&["jdk", "jdtls"]); // PATH prefix + JAVA_HOME for child processes
```

Lookup order: `path` configured in `toolchains.toml` → managed install matching the project
pin (when a version is pinned) → home variable (`JAVA_HOME`, `ANDROID_HOME`) → `PATH` → newest
managed install. Pins come from `.archipel/toolchains.toml` first, then `rust-toolchain(.toml)`,
`.bazelversion`, Gradle `JavaLanguageVersion.of(n)`/`jvmToolchain(n)`,
`gradle/gradle-daemon-jvm.properties`, `.sdkmanrc`, `.nvmrc`/`.node-version`,
`.python-version` and `go.mod`.

Language bundles describe components with a `ComponentSpec` (id, executables, bin dirs, home
variables, required components) and resolve versions to a `Release` (URL, SHA-256 or checksum
URL, archive kind) by implementing `ComponentProvider`; `StaticProvider` and `IndexProvider`
(JSON index, e.g. a corporate mirror) cover simple sources. Built-in providers live in
`archipel_ide_toolchains::providers`: `jdk` (foojay Disco API, Temurin unless `distribution` is
set), `jdtls` (Eclipse milestones), and GitHub-release tools (`rust-analyzer`, `bazelisk`,
`taplo`) verified with the asset `digest` or a published checksum file. Nothing downloads without consent:
installs start only from the `toolchain.install` command. Downloads resume with `Range`
requests, are verified before extraction (a mismatch deletes the file), extract into
`.staging/` and are renamed into place, so a version directory is either complete or absent.

Language servers reach managed installs through `archipel_ide_lsp::ProgramResolver`, a hook the
`LspManager` consults after an `lsp.toml` command and before `PATH` candidates;
`archipel-ide-core` implements it over `Toolchains` (`toolchain_lsp.rs`, e.g. jdtls runs on the
resolved JDK) and attaches the providing component to missing-server notices, whose **Set up**
action runs `toolchain.setup` and restarts the servers when it finishes.

Commands: `toolchain.list`, `toolchain.locate`, `toolchain.missing`, `toolchain.setup`
(component plus missing requirements), `toolchain.install`
(`wait: true` blocks, for the CLI), `toolchain.cancel`, `toolchain.jobs`, `toolchain.uninstall`,
`toolchain.usePath` ("use existing installation", stored in the user `toolchains.toml`) and
`toolchain.pins`. Running installs show in the status bar with a progress bar; clicking it cancels.
`ARCHIPEL_TOOLCHAINS_DIR` overrides the install root.

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
a command from `archipel_ide_core::keymap_commands::ACTION_COMMANDS` (IntelliJ action id →
command id, global or editor scope) or a workbench popup (`archipel-ide-ui::keybindings`).
To make a new command reachable by its reference shortcut, add its IntelliJ action id to
`ACTION_COMMANDS`; menus and Find Action then show the keymap's shortcut for it.

## Settings extension point

Settings pages are data, not UI code. A subsystem that needs a page registers it with
the registry held in `IdeState::settings` (normally from its `archipel-ide-core` command
module, before the UI starts):

```rust
use archipel_ide_settings::{PageDef, Scope, SettingDef, SettingKind, SettingValue};

let registry = &mut state.settings.registry;
// Reuse the reference page id when the page exists in docs/spec/data/settings.json
// (e.g. `reference.settingsdialog.project.gradle`); register a new page only otherwise.
registry.register_page(PageDef::new("archipel.rust", Some("language"), "Rust"))?;
registry.register_setting(
    SettingDef::new("rust.analyzerPath", "archipel.rust", "rust-analyzer executable",
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
`<config dir>/settings.toml` (application) or `<project>/.archipel/settings.toml`
(project); the config dir is `ARCHIPEL_CONFIG_DIR` or the platform default. Pages that
need bespoke UI (Keymap, Plugins) get a page renderer in `archipel-ide-ui::settings_dialog`.
Commands: `settings.open {page?}`, `settings.get/set/reset {id}`,
`settings.apply {changes}`, `settings.search {query}`.

## Long-term direction

The semantic platform layers VFS snapshots, incremental syntax, symbol/stub
indexes, dependency-aware invalidation, and prioritized cancellable jobs; the core of it
(`archipel-ide-analysis`) exists, see [analysis/architecture.md](analysis/architecture.md). LSP is one
provider behind a richer language-service interface, not the semantic architecture.
Untrusted compiled plugins use the WebAssembly Component Model with explicit capabilities
(`archipel-ide-plugin-wasm`); native plugins remain compiled-in and trusted.

## Reference findings

PhotoCraft validates the engine-first command-registry approach and generated parity
reports. IntelliJ's VFS/PSI/stub indexes motivate stable virtual identities and compact
externally-visible symbol indexes. Zed demonstrates GPU-first native UI; Lapce validates
rope-based text storage and language-service isolation. Archipel IDE reimplements behavior
cleanly and does not reuse protected assets or branding.

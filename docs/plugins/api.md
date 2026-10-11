# Jetthing plugin API (version 1)

Plugins add commands, menu items, keybindings, settings pages, tool windows,
editor tabs and status bar widgets to the workbench. A plugin can also run on its
own as a standalone app (`jetthing --app <plugin-id>`). Plugin API 1 covers
**compiled-in Rust plugins**, **Lua plugins** ([lua.md](lua.md)) and **WebAssembly component
plugins** ([wasm.md](wasm.md)).

| Crate | Layer | Purpose |
|---|---|---|
| `jetthing-plugin-api` | engine (no UI toolkit) | `Plugin` trait, `Contributions`, `PluginHost`, `PluginManifest`, `ViewFrame`, `testing::RecordingHost` |
| `jetthing-plugin-ui` | UI | `ui(frame) -> Option<&mut egui::Ui>`, `EguiView`, and theme-aware widgets (`header`, `section`, `property`, `empty_state`) |
| `jetthing-plugins` | engine | registry: discovery, enable/disable, settings store (`<config>/plugins.json`), activation order, `PluginLoader` for script runtimes, `.jtplugin` archives |
| `jetthing-plugin-wasm` | UI (runtime) | `WasmLoader`: wasmtime component plugins (`wit/plugin.wit`), WASI sandbox with permission-gated project files and network, fuel and memory limits |
| `jetthing-plugin-lua` | UI (runtime) | `LuaLoader`: sandboxed Lua 5.4 plugins with the same contributions, `ctx` services, immediate and declarative views |
| `jetthing-core` (`plugin_host`) | engine | `PluginHost` implementation, command routing, plugin tool window/tab state, `plugin.*` commands |
| `jetthing-ui` (`plugin_ui`, `plugin_manager`) | UI | renders contributions in the workbench, the Settings › Plugins page, standalone window |

The SDK ([`archipel-dev/sdk`](https://github.com/archipel-dev/sdk)) is the source of truth for
these crates: `jetthing-command`, `jetthing-plugin-api` and `jetthing-plugin-ui` only re-export
`archipel-command`, `archipel-plugin-api` and `archipel-plugin-ui`. Plugin tests can use the SDK's
`archipel-plugin-testing` (fake host, conformance and audit helpers). Manifests are named
`archipel-plugin.toml`; `jetthing-plugin.toml` (and legacy `plugin.toml`) are still read.

Engine crates never see egui. A view is rendered into an opaque `ViewFrame`, and
the egui host puts its `&mut egui::Ui` inside. `jetthing_plugin_ui::ui(frame)`
gives it back to the plugin.

## Minimal plugin

The complete example is [`plugins/hello`](../../plugins/hello/src/lib.rs). Its parts:

```toml
# plugins/my-tool/jetthing-plugin.toml
id = "com.example.mytool"        # reverse-domain, unique
name = "My Tool"
version = "0.1.0"
api_version = 1                  # must equal jetthing_plugin_api::API_VERSION
kind = "rust"                    # rust | lua | wasm | declarative
vendor = "Example"
description = "Shown on the Plugins page."
permissions = ["network"]        # network, process, fs.project, fs.user, clipboard, secrets
dependencies = []                # other plugin ids that must be active first

[app]                            # optional: enables `jetthing --app com.example.mytool`
view = "mytool.main"
title = "My Tool"
```

```toml
# plugins/my-tool/Cargo.toml
[package]
name = "jetthing-plugin-mytool"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
jetthing-plugin-api = { path = "../../crates/jetthing-plugin-api" }
jetthing-plugin-ui = { path = "../../crates/jetthing-plugin-ui" }
```

```rust
use jetthing_plugin_api::*;
use jetthing_plugin_ui::egui;

pub struct MyTool { manifest: PluginManifest, clicks: u32 }

impl MyTool {
    pub fn new() -> Self {
        Self {
            manifest: PluginManifest::parse(include_str!("../jetthing-plugin.toml"))
                .expect("bundled manifest is valid"),
            clicks: 0,
        }
    }
}

impl Plugin for MyTool {
    fn manifest(&self) -> &PluginManifest { &self.manifest }

    fn contributions(&self) -> Contributions {
        Contributions::new()
            .command(CommandContribution::new("mytool.click", "Click"))
            .menu(MenuLocation::Tools, "mytool.click")
            .keybinding("mytool.click", "Mod+Shift+K")   // Mod = ⌘ on macOS, Ctrl elsewhere
            .tool_window(ToolWindowContribution::new("mytool", "My Tool", "mytool.main"))
    }

    fn execute(&mut self, command: &str, _params: Value, host: &mut dyn PluginHost) -> CommandResult {
        match command {
            "mytool.click" => {
                self.clicks += 1;
                host.show_status(&format!("{} clicks", self.clicks));
                Ok(json!(self.clicks))
            }
            other => Err(CommandError::Unknown(other.into())),
        }
    }

    fn render_view(&mut self, _view: &ViewContext, frame: &mut ViewFrame<'_>, host: &mut dyn PluginHost) {
        let Some(ui) = jetthing_plugin_ui::ui(frame) else { return };
        if ui.button("Click").clicked() {
            host.dispatch("mytool.click", Value::Null); // or call your own method directly
        }
    }
}
```

Register the crate in two places:

1. `crates/jetthing-app/Cargo.toml`: add the dependency.
2. `crates/jetthing-app/src/plugins.rs`: add
   `plugins.push(Rc::new(|| Box::new(my_tool::MyTool::new())));`.

The workspace picks up `plugins/*` crates automatically.

## Lifecycle

1. **Discovery.** At startup the registry adds the bundled factories. It then reads
   manifests from the per-user plugin directory (`<config>/plugins/*/jetthing-plugin.toml`)
   and from the open project's `plugins/` directory. `<config>` is `$JETTHING_CONFIG_DIR`,
   or `~/.config/jetthing`, `~/Library/Application Support/jetthing` or `%APPDATA%\jetthing`.
   Invalid manifests are listed with their error in File ▸ Plugins….
2. **Activation.** Enabled plugins activate in dependency order. `activate(host)` runs first;
   then `contributions()` is read and validated (`Contributions::problems()`). If activation
   fails, or the contributions are inconsistent, the plugin is marked *Failed* with the reason.
3. **Running.** Contributed commands go into the shared command registry. That makes them
   available in Search Everywhere / Find Action, the main menu, keybindings and the CLI
   surface, and they run through `Plugin::execute`. `Plugin::poll` runs every frame;
   collect background results there and call `host.request_repaint()` while work is in flight.
4. **Disable / enable.** This works without a restart. `deactivate(host)` runs, the plugin's
   commands are unregistered and its tool window and tabs close. Re-enabling creates a fresh
   instance. The choice persists in `<config>/plugins.json`.
5. **Exit.** `deactivate` runs for every active plugin.

All plugin methods run on the UI thread. Never block in them: spawn a thread
(or a runtime) and hand results back through `poll`.

## Contributions

| Contribution | Builder | Where it appears |
|---|---|---|
| Command | `CommandContribution::new(id, label).category(..).params(..)` | Search Everywhere, Find Action, CLI; ids must be namespaced (`plugin.action`) and must not clash with built-ins (built-ins win) |
| Menu item | `.menu(MenuLocation::Tools, command)` | main menu (File, Edit, View, Navigate, Code, Build, Run, Git, Tools, Help) |
| Keybinding | `.keybinding(command, "Mod+Alt+H")` or `"⌥⌘H"` | global, after built-in shortcuts; also shown as the command's shortcut |
| Settings page | `SettingsPage::new(id, title).field(SettingField::new(key, label, SettingKind::…))` | Settings tab of the plugin's details on the Plugins page; values via `host.setting(key)`; `Text`, `Bool`, `Number{min,max}` (clamped), `Choice` |
| Tool window | `ToolWindowContribution::new(id, title, view).icon_text("H")` | right tool window stripe and panel |
| Editor view | `EditorContribution::new(view, title)`; open with `host.open_editor_tab(EditorTabRequest::new(view, title).key(..))` | editor tab bar; the same key focuses the existing tab |
| Status widget | `StatusWidgetContribution::new(id).command(..)` | status bar; text from `Plugin::status_widget(id)` |
| App view | `[app] view = …` in the manifest | `jetthing --app <id>` window |

Views can also return a declarative `ViewNode` tree (columns, rows, labels, buttons, inputs,
lists, tables, trees) instead of drawing with egui. `jetthing_plugin_ui::render_node` renders it
and returns `ViewEvent`s. Lua and WASM plugins use this model.

`ViewContext::placement` tells a view where it is rendered:
`ToolWindow { id }`, `EditorTab { key, path }`, `App` or `SettingsPage { id }`.

## Host services (`PluginHost`)

| Method | Notes |
|---|---|
| `plugin_id()`, `project_root()` | |
| `show_status(msg)`, `notify(Notification::info/error)` | notifications show as balloons |
| `dispatch(command, params)` | queued; runs after the current call (built-in or contributed commands) |
| `open_tool_window(id)`, `open_editor_tab(request)` | own views only |
| `setting(key)`, `set_setting(key, value)` | settings-page defaults apply when unset |
| `data_dir()` | `<config>/plugin-data/<id>`, created on demand |
| `has_permission(p)`, `require_permission(p)` | checks the manifest's `permissions` |
| `refresh_contributions()` | re-reads `contributions()` after the call |
| `request_repaint()` | keeps the UI animating while busy |

## Testing

`jetthing_plugin_api::testing::RecordingHost` records statuses, notifications,
dispatched commands, opened views and settings. Views can be rendered headlessly
with `egui::Context::default().run(...)`. See the tests in `plugins/hello`.

## Commands for plugin management

| Command | Params |
|---|---|
| `plugin.manager.open` | — open the Plugins page (File ▸ Plugins…) |
| `plugin.install` | `{"path"}` — install a `.jtplugin` archive |
| `plugin.uninstall` | `{"id"}` — remove an installed plugin |
| `plugin.refresh` | — rediscover manifests |
| `plugin.list` | — JSON list with status and failure reason |
| `plugin.enable`, `plugin.disable` | `{"id"}` |
| `plugin.setSetting` | `{"plugin","key","value"}`, validated against the settings page |
| `plugin.toolwindow.toggle` | `{"plugin","id"}` |
| `plugin.tab.activate`, `plugin.tab.close` | `{"index"}` (close defaults to the active tab) |

## Packaging and installing (`.jtplugin`)

A `.jtplugin` file is a zip archive. `jetthing-plugin.toml` sits at its root, or inside one
top-level directory, next to the plugin's files.

```bash
cargo run -p xtask -- pack-plugin path/to/plugin [out.jtplugin]   # default: dist/plugins/<id>-<version>.jtplugin
```

Hidden files and `target/` are skipped. Install from **File ▸ Plugins… ▸ ⚙ ▸ Install Plugin from
Disk…** or with `plugin.install {"path"}`. The archive is validated before anything is written.
Installation rejects archives that:

- have an invalid manifest, more than one plugin, or a missing `entry`;
- contain entries outside the plugin directory, or symbolic links;
- have more than 10 000 entries or unpack to more than 512 MiB;
- contain a Rust plugin (Rust plugins are compiled in).

Files are extracted to a staging directory first, then swapped into
`<config>/plugins/<id>/`. Reinstalling replaces the previous version without a restart.
`plugin.uninstall {"id"}` (or **Uninstall** on the Plugins page) deactivates the plugin and removes
its directory. Bundled and project plugins cannot be uninstalled, and an archive cannot replace a
bundled plugin's id.

## Standalone app mode

`jetthing --app <plugin-id>` (or `--app=<id>`) opens a window that contains only the
plugin's `[app]` view, with a title bar, window controls, status text, the plugin's
status widgets and its notifications. An unknown, disabled or app-less plugin exits
with code 2 and a message that lists the plugins that do have an app view.

## Stability

API 1 is stable once the first-party plugins (nexus-http, mongo-raabit) build on
it. From then on, breaking changes bump `API_VERSION` and are announced in the
PR body. Additive changes (new contribution kinds, new host methods with default
implementations) do not.

## Roadmap

- Move the Plugins page into the Settings dialog, and add an update check for installed plugins.
- Left/bottom tool window anchors for plugins (API 1 docks every plugin tool window on the right),
  editor/project context menus, and opening custom editors by file extension.
- JetBrains Marketplace browsing, and conversion of themes, color schemes, keymaps,
  live templates, TextMate bundles and LSP plugins.

# Plugin system evidence (API 1)

Captured under Xvfb 1600×1000, `ARCHIPEL_RENDERER=glow`. The build used
`cargo build -p archipel-ide-app --features example-plugins` with an empty
`ARCHIPEL_CONFIG_DIR`.

| File | Shows |
|---|---|
| `hello-toolwindow-and-tab.png` | Example plugin: "H" button on the right stripe with the Hello tool window open, a plugin editor tab ("Hello") next to a file tab, the plugin notification balloon and the plugin status widget. "Open Hello Tab" ran from Find Action, where plugin commands are listed with category and shortcut. |
| `24-plugins-installed.png` | File ▸ Plugins… (`plugin.manager.open`) right after ⚙ ▸ Install Plugin from Disk… installed `darcula-plus.archipelplugin` (packed with `cargo xtask pack-plugin`): Marketplace / Installed tabs with count badge and gear; search field; groups "Downloaded (1 of 1 enabled)", "Project" and "Bundled" with Disable all links; rows with icon, bold name, version and vendor, checkbox; details with vendor, homepage link, Disable / Uninstall, version, Overview / Additional Info tabs. |
| `24-plugins-settings-tab.png` | Same page with the bundled Hello plugin selected: the Settings tab renders its declarative settings page (text and boolean fields). |
| `24-plugins-marketplace.png` | Marketplace tab: no marketplace is configured yet, so an empty state offers Install Plugin from Disk…. |
| `standalone-app.png` | `archipel-ide --app dev.archipel.hello`: only the plugin's app view, with title bar, window controls, status text, status widget and notification. |

| `lua-plugin-toolwindow-and-declarative-tab.png` | `plugins/hello-lua`, discovered from the open project's `plugins/` directory: "L" tool window with the immediate-mode view (text input, buttons, properties) and the "Todos (Lua)" editor tab with a declarative view (text input + Add, list toggled by clicking, muted label). Two todos added, one toggled. The status bar shows the Lua status widget ("Lua × 1"); a Lua notification is showing. |
| `lua-standalone-app-with-tab.png` | `--app dev.archipel.hello-lua` (installed from `hello-lua.archipelplugin`): the app view plus the Todos tab it opened; the status bar shows only this plugin's widget. |

| `wasm-plugin-toolwindow-project-permission.png` | `plugins/hello-wasm` (a 146 KB `wasm32-wasip2` component) in its "W" tool window: declarative view with a name field and Say Hello, statistics, and `List /project` showing the project root read through the WASI preopen granted by `permissions = ["fs.project"]`. The status bar shows "WASM × 1". |
| `24-plugins-wasm-additional-info.png` | Plugins page listing Rust (bundled), Lua, WASM and declarative plugins; Hello WASM's Additional Info shows kind `wasm` and the permission "Read and write files in the open project". |

Also observed in the same session:

- Uninstall removed `<config>/plugins/com.example.darcula-plus/` and the entry; the status bar said
  "Uninstalled plugin com.example.darcula-plus".
- Unchecking Hello removed its stripe button, status widget and commands at once, and
  wrote `{"disabled":["dev.archipel.hello"]}` to `plugins.json`. Re-checking restored them.
- `archipel-ide --app no.such` printed
  `archipel-ide: unknown plugin `no.such`; plugins with an app view: dev.archipel.hello`
  and exited with code 2.
- Alt-based chords could not be sent through xdotool in this Xvfb setup; the built-in
  Alt+1 failed the same way. Ctrl chords work. Keybinding mapping is covered by unit tests
  (`plugin_ui::tests`, `contributions::tests`).

## Comparison with reference scene 24-plugins

Reference: `reference/intellij-community/java-gradle/24-plugins.png` (Marketplace tab) and
`reference/intellij-community/bazel/b07-plugin-installed.png` (Installed tab).

Matches: header "Plugins" with Marketplace / Installed tabs, an Installed count badge and a gear menu
(Install Plugin from Disk…); search hint "Type / to see options"; grouped list with grey group headers,
"Downloaded (x of y enabled)" and "Disable all" links; 64 px rows with icon, bold name, version/vendor
line and checkbox; details pane with name, vendor, homepage link, Enable/Disable button, version and
Overview / Additional Info tabs; Uninstall for downloaded plugins.

Differences:
- Integration 4B moved the page into the Settings dialog as its Settings › Plugins page
  (`preferences.pluginManager`, the page the settings registry registers from the reference data),
  replacing both the standalone window and train 3's simpler Plugins page. `plugin.manager.open` and
  File › Plugins… open the dialog on it. The screenshots above were taken of the standalone window.
- Plugin changes (enable/disable, install, uninstall, plugin settings) apply immediately, not on
  OK / Apply.
- The Marketplace tab has no catalog, so there are no Suggested / Staff Picks sections, downloads,
  ratings, What's New or Reviews tabs, and no screenshots carousel.
- Icons are generated initials (no plugin artwork), there is no "Bundled plugins updates" section,
  and Disable is a plain button rather than a split button.
- Uninstalling happens immediately; there is no "restart required" state, because nothing in the
  current plugin kinds needs a restart.

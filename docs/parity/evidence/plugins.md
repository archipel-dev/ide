# Plugins settings page (scene 24, PLAT-034)

Reference: `reference/intellij-community/java-gradle/24-plugins.png`. Archipel IDE:
`archipel-ide/plugins/24-plugins-installed.png`, `24-plugins-failed-manifest.png`,
`24-plugins-marketplace.png`.

## Steps

1. Put a valid (`go/plugin.toml`, id `dev.example.go`) and an invalid (`broken/`, id
   `bad`) manifest under `$ARCHIPEL_CONFIG_DIR/plugins/`, then start Archipel IDE.
2. Ctrl+Alt+S › Plugins: the tabs read "Marketplace" and "Installed 3". The list shows Rust Language Support
   (0.1.0, Bundled), broken (Failed to load) and Go Support (0.2.0, User).
3. Select broken → `24-plugins-failed-manifest.png`: "This plugin failed to load", the
   error ("plugin id must use reverse-domain syntax: bad") and the manifest path.
4. Uncheck Go Support, then select Rust → `24-plugins-installed.png`: the detail pane shows
   the source tag, name, id, Enabled, version, capabilities (language lsp build test),
   API version and manifest path. `settings.toml` stores
   `"plugins.disabled" = ["dev.example.go"]`.
5. Marketplace → `24-plugins-marketplace.png`: "No plugin marketplace yet" and the
   plugin directories in the order they are searched.

## Differences from the reference

The reference Marketplace (Suggested, Staff Picks, Install buttons, ratings, screenshots,
Overview / What's New / Reviews) has no Archipel IDE counterpart: there is no marketplace
service and plugins cannot run code yet. Toggling a plugin applies immediately; the
reference applies it on OK/Apply and asks for a restart.

## Integration note

The screenshots were taken on the branch before it was integrated with the plugin API
(PLAT-006). In the integrated build the page lists the plugin registry: compiled-in
plugins (Rust Language Support, Hello) and `archipel-plugin.toml` / `plugin.toml`
manifests from the plugin directories. The checkbox runs `plugin.enable` / `plugin.disable`,
which the plugin store persists, instead of the `plugins.disabled` setting.

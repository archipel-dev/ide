# First run: Customize Archipel IDE

Reference: on a first start (no previous configuration), IntelliJ-based IDEs
offer to import settings from a previous installation and to customize the UI
theme and keymap before the welcome screen.

## Archipel IDE behaviour

- **When it opens.** At startup the wizard opens if the config directory
  (`archipel_ide_settings::default_config_dir`, `ARCHIPEL_CONFIG_DIR` overrides it)
  contains neither `settings.toml` nor the `first-run.done` marker. Existing users
  who already have settings are not asked. **Help → Customize Archipel IDE…**
  (`app.customize`) reopens it.
- **Choices.**
  - **Theme:** Islands Dark or Islands Light.
  - **Keymap:** the option list of `keymap.active`, defaulting to the platform keymap.
  - **Import settings (optional):** a path to a `settings.toml` from another
    installation.
- **Start Using Archipel IDE** (`app.completeCustomize`, with the parameters `theme`,
  `keymap` and `importFrom`):
  - It validates the choices before changing anything, so an invalid choice leaves
    the wizard open.
  - It imports every known and valid setting from the file. Unknown ids and invalid
    values are skipped and reported.
  - Explicit choices win over imported values.
  - It persists the settings, writes the marker, and shows the result in the
    status bar.
- **Skip or Escape** (`app.skipCustomize`) writes the marker only.
- The theme switches live, because the UI reads `appearance.theme` from the
  settings store each frame. The control socket reports `popups.customize`.

## Reproduction (Linux, Xvfb)

```text
$ export ARCHIPEL_CONFIG_DIR=/tmp/jt-cfg ARCHIPEL_CONTROL_SOCKET=/tmp/jt.sock
$ DISPLAY=:99 ARCHIPEL_RENDERER=glow ARCHIPEL_NO_UPDATE_CHECK=1 archipel-ide &
$ archipel-cli ui state            # popups.customize == true
$ archipel-cli ui screenshot first-run-customize.png
$ archipel-cli ui execute app.completeCustomize '{"theme":"Islands Light","keymap":"Default for KDE"}'
{"imported": [], "skipped": []}
$ archipel-cli ui state            # popups.customize == false, status "Archipel IDE is set up"
$ ls /tmp/jt-cfg && cat /tmp/jt-cfg/settings.toml
first-run.done  settings.toml
[settings]
"appearance.theme" = "Islands Light"
"keymap.active" = "Default for KDE"
```

| First start | After "Start Using Archipel IDE" with Islands Light |
|---|---|
| ![Customize Archipel IDE](first-run-customize.png) | ![Light theme applied](first-run-light.png) |

Tests (`archipel-ide-core::first_run`):
- the wizard opens only without previous settings or the marker;
- completing applies and persists the theme and keymap, and an invalid choice
  keeps the wizard open;
- import takes valid settings, reports unknown ones, lets explicit choices win,
  and errors on a missing file;
- skip writes the marker, and `app.customize` reopens the wizard.

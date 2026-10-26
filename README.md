# Archipel IDE

Archipel IDE is an experimental, clean-room, native Rust IDE platform with a workbench
modelled on the observable layout of the IntelliJ New UI ("Islands Dark"): merged main
toolbar with project, VCS and run widgets, a tool window stripe, rounded editor and tool
window islands, Search Everywhere, and a status bar with navigation breadcrumbs.

![Archipel IDE debugging a Gradle Java project](docs/parity/evidence/debugger/146-java-step-over.png)

> **Formerly Jetthing.** Environment variables (`ARCHIPEL_*`), the per-project `.archipel/`
> directory, per-user config/log/cache directories, plugin archives and plugin ids were
> renamed. Data and settings of earlier versions are still read or migrated, and the old
> environment variable names still work with a one-time warning; see
> [Compatibility with the Jetthing names](docs/architecture.md#compatibility-with-the-jetthing-names).

## Run

```bash
cargo run -p archipel-ide-app                           # welcome screen, Open Project
cargo run -p archipel-ide-app -- .                      # open a project
cargo run -p archipel-ide-app -- . src/main.rs          # open a project and files
cargo run -p archipel-ide-app --features example-plugins -- --app dev.archipel.hello
                                                    # a plugin as a standalone app
cargo run -p archipel-ide-cli -- commands               # headless command discovery
cargo run -p archipel-ide-app -- --version              # version, commit, date, target
cargo run -p xtask -- parity                        # regenerate the parity dashboard
```

If wgpu finds no usable adapter (VMs, old drivers) the app falls back to OpenGL
automatically; `ARCHIPEL_RENDERER=glow` forces it.

On Linux the window needs the X11/Wayland client libraries, for example on Debian/Ubuntu
`libxkbcommon-x11-0` (X11) or `libwayland-client0` (Wayland), plus OpenGL/Vulkan drivers.
A missing library is reported with the package to install.

Use **New Project** on the welcome screen or in File to generate an Empty, Rust,
or Java/Gradle project. Choose an existing parent folder and a new project name.
Java templates require JDK 21 and Gradle on PATH; Rust templates require Cargo.

## Install

Download from the [latest release](https://github.com/archipel-dev/ide/releases/latest):

| Platform | Installer | Portable archive |
|---|---|---|
| Windows | `archipel-ide-<version>-windows-x86_64.msi` | `….zip` |
| macOS (Apple silicon / Intel) | `archipel-ide-<version>-macos-aarch64.dmg` / `…-x86_64.dmg` | `….tar.gz` |
| Linux | `archipel-ide_<version>_amd64.deb` (`sudo apt install ./archipel-ide_*.deb`) | `….tar.gz` with `.desktop` file |
| Linux (any distribution) | `archipel-ide-<version>-linux-x86_64.AppImage` (`chmod +x`, run) | |

- **Windows:** run the MSI. It installs to Program Files, adds a Start Menu entry and
  puts `archipel-ide` on `PATH`. Uninstall it from **Installed apps**.
- **macOS:** drag Archipel IDE from the DMG to Applications. To uninstall, move it to the
  Bin.
- **Linux:** install the `.deb` with the command in the table. Uninstall it with
  `sudo apt remove archipel-ide`.

Homebrew cask, winget and Scoop manifests are generated with every release build
(`cargo xtask manifests`); see [`docs/release.md`](docs/release.md#package-managers).

Builds are not signed by default, so Windows SmartScreen and macOS Gatekeeper warn on
first launch. [`docs/release.md`](docs/release.md) explains how to proceed, and covers
checksums, silent installs and optional signing.

## UI automation

Set `ARCHIPEL_CONTROL_SOCKET=/path/ctl.sock` to let scripts drive a running IDE over
JSON lines: run commands, read state, send keys, text and clicks, wait, and take screenshots.
On Windows the value names a named pipe (`archipel-ide` → `\\.\pipe\archipel-ide`).

```bash
ARCHIPEL_CONTROL_SOCKET=/tmp/jt.sock cargo run -p archipel-ide-app -- . &
export ARCHIPEL_CONTROL_SOCKET=/tmp/jt.sock
cargo run -p archipel-ide-cli -- ui wait idle
cargo run -p archipel-ide-cli -- ui key Ctrl+Shift+A
cargo run -p archipel-ide-cli -- ui screenshot /tmp/find-action.png
cargo run -p archipel-ide-cli -- ui state
```

See `docs/parity/evidence/ui-control-socket.md` for the protocol.
## Plugins

Plugins add commands, menu items, keybindings, settings, tool windows, editor
tabs and status bar widgets. Manage them in **File → Plugins…**. Run a plugin
that declares an `[app]` view on its own with `archipel-ide --app <plugin-id>`. See
`docs/plugins/api.md`; `plugins/hello` is the example plugin (bundled with
`--features example-plugins`).

## Builds, releases and updates

```bash
cargo run -p xtask -- dist               # archive in dist/ (the self-updater's format)
cargo run -p xtask -- dist --installer   # + .msi / .dmg / .deb for the host OS
cargo run -p xtask -- changelog          # release notes since the last v* tag, from conventional commits
```

`.github/workflows/release.yml` builds and smoke-tests the archives and installers for
Linux, Windows and macOS (arm64 and x86_64), each with a `.sha256` checksum. A push to
`main` publishes GitHub release `v<version>` when that release is missing or
incomplete. Pushing tag `v<version>` publishes or repairs it. To ship a new version,
bump `[workspace.package] version` and merge.

The IDE checks for a newer release on startup and via **Help → Check for Updates…**.
For archive installs, **Update and Restart** downloads the archive, verifies its
checksum and swaps it in place of the running executable. The previous one is kept as
`archipel-ide.old`. For MSI, DMG and `.deb` installs, it downloads and verifies the new
installer instead, and **Quit and Install** hands it to the OS. Development builds
(`cargo run`) only link to the release page. Details and environment variables
(`ARCHIPEL_NO_UPDATE_CHECK`, `ARCHIPEL_GITHUB_TOKEN`, `ARCHIPEL_UPDATE_REPOSITORY`)
are in [`docs/release.md`](docs/release.md).

## Logs and crash reports

`archipel.log` lives in `~/.local/state/archipel/logs` (Linux), `~/Library/Logs/Archipel`
(macOS) or `%LOCALAPPDATA%\Archipel\logs` (Windows); `ARCHIPEL_LOG_DIR` overrides it.
**Help → Show Log in Files** opens it. A crash writes `crash-*.txt` there (build, panic,
backtrace, log tail) and the next start offers it in a notification; attach it to bug
reports together with **Help → About → Copy**.

## Keyboard

Shortcuts come from the reference IDE's bundled keymaps (`docs/spec/data/keymaps.json`).
The default follows the desktop (macOS, Windows, GNOME, KDE, XWin); pick another keymap or
change shortcuts in **Settings › Keymap**. Common defaults (macOS / Windows & Linux):

| Action | macOS | Windows / Linux |
|---|---|---|
| Search Everywhere | Double ⇧ | Double Shift |
| Find Action | ⇧⌘A | Ctrl+Shift+A |
| Settings | ⌘, | Ctrl+Alt+S |
| Project / Commit tool window | ⌘1 / ⌘0 | Alt+1 / Alt+0 |
| Find in Files | ⇧⌘F | Ctrl+Shift+F |
| Terminal | ⌥F12 | Alt+F12 |
| Run / Build | ⌃R / ⌘F9 | Shift+F10 / Ctrl+F9 |
| Close tab | ⌘W | Ctrl+F4 |
| Hide all tool windows | ⇧⌘F12 | Ctrl+Shift+F12 |
| Go to File / Recent Files | ⇧⌘O / ⌘E | Ctrl+Shift+N / Ctrl+E |
| Find / Replace in file | ⌘F / ⌘R | Ctrl+F / Ctrl+R |
| Go to Line | ⌘L | Ctrl+G |
| Duplicate / Delete line | ⌘D / ⌘⌫ | Ctrl+D / Ctrl+Y |
| Comment line | ⌘/ | Ctrl+/ |
| Move line up / down | ⌥⇧↑ / ⌥⇧↓ | Alt+Shift+↑ / Alt+Shift+↓ |
| Commit | ⌘K | Ctrl+K |
| Rename / Delete in Project tree | ⇧F6 / ⌦ | Shift+F6 / Delete |

## AI Assistant

AI features are off until you enable a provider in the **AI Chat** tool window (right
stripe) or with `ai.configure`. Supported: Anthropic (default models `claude-opus-5-5`
for chat and `claude-haiku-5-5` for completion), any OpenAI-compatible endpoint, and a
local Ollama server. Keys are read from the Archipel IDE config directory (`secrets.toml`,
owner-only) or from `ANTHROPIC_API_KEY` / `OPENAI_API_KEY`. A project can opt out or
stop file sharing in `.archipel/ai.toml` (`allow_ai`, `share_file_contents`).

Inline completion shows ghost text at the end of a line after a short typing pause:
Tab accepts, Ctrl+→ (⌥→ on macOS) accepts a word, Esc dismisses. Turn it off with
`ai.inline.setEnabled`.

| Variable | Effect |
|---|---|
| `ARCHIPEL_CONFIG_DIR` | config directory for `ai.toml` and `secrets.toml` |
| `ARCHIPEL_AI_MOCK` | offline demo provider with canned answers (nothing is sent) |

## Assets and trademarks

UI colors are facts extracted from IntelliJ Community theme files (Apache-2.0, see
`docs/spec/data/NOTICE.md`). Fonts are Inter and JetBrains Mono under the SIL Open Font
License (`crates/archipel-ide-ui/assets/fonts/`). Icons, including the app icon
(`assets/icon/archipel.svg`), are original vector drawings; no JetBrains icons or
artwork are bundled. IntelliJ and JetBrains are trademarks of
JetBrains s.r.o.; Archipel IDE is not affiliated with or endorsed by JetBrains.

See `docs/roadmap.md` for honest status and limitations.

<!-- parity:begin -->
**Parity progress:** 24.8% implemented (weighted), 0.0% verified, across 788 tracked
features in 19 areas. See [`docs/parity/dashboard.md`](docs/parity/dashboard.md).
<!-- parity:end -->

# Native installers (PLAT-028)

How to reproduce: `cargo run -p xtask -- dist --installer` on each OS (see
`docs/release.md`), or run the **Release** workflow manually on any branch.

## Linux `.deb` (local, Ubuntu container, x86_64)

```text
$ cargo run -p xtask -- dist --installer
dist/archipel-ide-0.0.1-linux-x86_64.tar.gz
dist/archipel-ide_0.0.1_amd64.deb
$ sha256sum -c dist/*.sha256
archipel-ide-0.0.1-linux-x86_64.tar.gz: OK
archipel-ide_0.0.1_amd64.deb: OK
$ sudo dpkg -i dist/archipel-ide_0.0.1_amd64.deb
Setting up archipel-ide (0.0.1) ...
Processing triggers for hicolor-icon-theme (0.17-2) ...
$ readlink -f /usr/bin/archipel-ide
/usr/lib/archipel-ide/archipel-ide
$ archipel-cli commands | head -1
[
$ sudo dpkg -r archipel-ide && ls /usr/lib/archipel-ide /usr/bin/archipel-ide
ls: cannot access '/usr/lib/archipel-ide': No such file or directory
ls: cannot access '/usr/bin/archipel-ide': No such file or directory
```

## Linux AppImage (local, Ubuntu container, x86_64)

```text
$ APPIMAGETOOL=…/appimagetool-1.9.0 cargo run -p xtask -- dist --installer
dist/archipel-ide-0.0.2-linux-x86_64.tar.gz
dist/archipel-ide_0.0.2_amd64.deb
dist/archipel-ide-0.0.2-linux-x86_64.AppImage
$ APPIMAGE_EXTRACT_AND_RUN=1 ./dist/archipel-ide-0.0.2-linux-x86_64.AppImage --version
archipel-ide 0.0.2 (ab6c5034e2d2 2026-10-10, x86_64-unknown-linux-gnu, release)
$ ./dist/archipel-ide-0.0.2-linux-x86_64.AppImage --appimage-extract && ls -A squashfs-root
.DirIcon  AppRun  archipel-ide.desktop  archipel.png  usr
```

## Package-manager manifests

`cargo xtask manifests --dir <artifacts>` renders a Homebrew cask, winget manifests
(1.6.0) and a Scoop manifest. In local checks, the YAML parsed with PyYAML, the JSON
with `json.load`, and the cask passed `ruby -c` (`Syntax OK`).

## CI (GitHub-hosted runners)

The `Release` workflow builds every archive and installer, verifies the checksums and
smoke-tests each installer:

- **ubuntu-22.04**: `.deb` install, CLI run, then uninstall leaves nothing behind.
- **windows-latest**: silent MSI install. The workflow checks Program Files, the
  install marker, the Start Menu shortcut, the CLI and the Add/Remove Programs entry.
  A reinstall must still show a single product (MajorUpgrade), and uninstall must
  leave nothing behind.
- **macos-latest**, arm64 and x86_64: the workflow mounts the DMG and checks the
  Applications link, `plutil -lint`, `codesign --verify --deep --strict`, `lipo`
  architectures and a CLI run on the native architecture.

First green run: [Release #12](https://github.com/archipel-dev/ide/actions/runs/38059816135)
(workflow_dispatch on `feat/native-installers`, commit `bfd2745`, version 0.0.2;
`release` job skipped as intended off `main`). Every job passed: `checks`,
`package linux-x86_64`, `package windows-x86_64`, `package macos-aarch64` and
`package macos-x86_64`, including each smoke-test step. Artifacts:

| Artifact | Contents |
|---|---|
| `archipel-ide-0.0.2-linux-x86_64` | `archipel-ide-0.0.2-linux-x86_64.tar.gz`, `archipel-ide_0.0.2_amd64.deb`, `.sha256` each |
| `archipel-ide-0.0.2-windows-x86_64` | `archipel-ide-0.0.2-windows-x86_64.zip`, `archipel-ide-0.0.2-windows-x86_64.msi`, `.sha256` each |
| `archipel-ide-0.0.2-macos-aarch64` | `archipel-ide-0.0.2-macos-aarch64.tar.gz`, `archipel-ide-0.0.2-macos-aarch64.dmg`, `.sha256` each |
| `archipel-ide-0.0.2-macos-x86_64` | `archipel-ide-0.0.2-macos-x86_64.tar.gz`, `archipel-ide-0.0.2-macos-x86_64.dmg`, `.sha256` each |

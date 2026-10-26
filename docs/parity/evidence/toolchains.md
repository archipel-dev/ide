# Toolchain provisioning evidence

Issue #75. Reference behaviour: IntelliJ IDEA's *Download JDK…* (Project Structure → SDKs)
downloads a vendor JDK chosen from the foojay Disco API into `~/.jdks/<vendor>-<version>`,
shows progress in the status bar and detects JDKs that are already installed
(`JAVA_HOME`, `/usr/lib/jvm`, SDKMAN) before offering a download — see
`reference/intellij-community/NOTES.md` (project JDK auto-detected as
`Java 21.0.12 /usr/lib/jvm/java-1.21.0-openjdk-amd64`). Archipel IDE generalises this to every
component a language needs, under `<data dir>/archipel/toolchains/<component>/<version>/`.

## Slice 1 — provisioning core (this evidence)

Everything below runs headless against a local fake HTTP server
(`crates/archipel-ide-toolchains/tests/support/mod.rs`: fixed bodies, `Range: bytes=n-` with
`206`/`416`, one-shot connection cuts, request log). Reproduce with:

```bash
cargo test -p archipel-ide-toolchains
cargo test -p archipel-ide-core toolchain
```

| Acceptance item (#75) | Test |
|---|---|
| Resolution (index, version prefix, channel, platform) | `resolves_downloads_verifies_installs_and_uninstalls`, `catalog::tests::resolves_versions_channels_and_platforms` |
| Checksum verification (inline digest and `sha256sum` file) | `resolves_downloads_verifies_installs_and_uninstalls` |
| Checksum mismatch → nothing installed, download deleted | `checksum_mismatch_installs_nothing_and_discards_the_download` |
| Resume: second attempt sends `Range: bytes=<cut>-` and continues from the cut | `interrupted_download_resumes_with_a_range_request` |
| Atomic install: a broken archive leaves no version dir and no staging leftovers | `failed_extraction_leaves_no_partial_install` |
| Uninstall; locate falls back to the remaining version | `resolves_downloads_verifies_installs_and_uninstalls` |
| PATH-first discovery; home variable; pinned managed version; configured path | `path_and_home_installs_win_unless_the_project_pins_a_managed_version` |
| Offline: clear error naming host and proxy settings; HTTP status errors | `unreachable_hosts_report_offline_and_missing_files_report_http_status` |
| Background install with progress events; duplicate requests join the job | `service_installs_in_the_background_and_reports_progress` |
| Archive safety: `../` entries rejected, tar.gz/tar.xz/zip/gz/raw layouts | `extract::tests::*` |
| Project pins from version files | `pinning::tests::*` |
| `NO_PROXY` hosts, suffixes, IPv4 CIDR | `download::tests::no_proxy_matches_hosts_suffixes_and_networks` |
| Commands: install (wait), locate, list, missing, uninstall, usePath, pins, errors | `toolchain_commands::tests::*` |

Results on Linux x86_64 (2026-10-10): 17 unit + 7 integration tests in
`archipel-ide-toolchains`, 3 command tests in `archipel-ide-core`, all passing.

CLI smoke run (no catalog entries ship in this slice, so nothing is installable yet):

```text
$ ARCHIPEL_TOOLCHAINS_DIR=/tmp/tc archipel-cli run toolchain.list
{"components":[],"platform":"linux-x86_64","root":"/tmp/tc","settingsError":null}
$ archipel-cli run toolchain.install '{"component":"jdk"}'
invalid parameters: unknown toolchain component `jdk`
```

## Slice 3a — first catalog entries

Built-in providers (`crates/archipel-ide-toolchains/src/providers/`): `jdk` (foojay Disco API),
`jdtls` (download.eclipse.org milestones), `rust-analyzer`, `bazelisk` and `taplo` (GitHub
releases with asset digests). Provider tests: `cargo test -p archipel-ide-toolchains --test providers`.

Live run on a Linux x86_64 container, 2026-10-10. A system JDK 21 is present
(`JAVA_HOME=/usr/lib/jvm/java-21-openjdk-amd64`), so `toolchain.list` correctly keeps
preferring it while the managed install exists alongside:

```text
$ export ARCHIPEL_TOOLCHAINS_DIR=/tmp/tcroot
$ archipel-cli run toolchain.install '{"component":"jdk","version":"21","wait":true}'
{"component":"jdk","job":1,"path":"/tmp/tcroot/jdk/21.0.12.1+1","version":"21.0.12.1+1"}   # 16.7 s
$ /tmp/tcroot/jdk/21.0.12.1+1/bin/java -version
openjdk version "21.0.12.1" 2026-08-18 LTS
$ cat /tmp/tcroot/jdk/21.0.12.1+1/.archipel-install.json
{ "component": "jdk", "version": "21.0.12.1+1",
  "source_url": "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.12.1%2B1/OpenJDK21U-jdk_x64_linux_hotspot_21.0.12.1_1.tar.gz",
  "sha256": "ce79869e1307ed8ee1e2baa86a412b1eb5b75d10a01006d788a6f968bcfaee94", ... }
$ archipel-cli run toolchain.install '{"component":"jdtls","wait":true}'
{"component":"jdtls","job":1,"path":"/tmp/tcroot/jdtls/1.61.0","version":"1.61.0"}       # 5.9 s
$ archipel-cli run toolchain.list
jdk:   located JAVA_HOME (/usr/lib/jvm/java-21-openjdk-amd64), installed ["21.0.12.1+1"]
jdtls: located managed (/tmp/tcroot/jdtls/1.61.0/bin/jdtls), installed ["1.61.0"], requires ["jdk"]
rust-analyzer: located PATH (/root/.cargo/bin/rust-analyzer)
```

(`toolchain.list` output abbreviated from JSON.) In this sandbox the GitHub REST API is
blocked by the network policy, so no live rust-analyzer/bazelisk/taplo install is recorded;
their release assets themselves download fine.

## Slice 4 — LSP integration and the Java end-to-end case

`archipel-ide-lsp` gained a minimal hook: `ProgramResolver` (`resolve(spec)` →
`ResolvedProgram { program, args, env }`, `setup_component(spec)`), consulted after an
`lsp.toml` command and before `PATH` candidates. `archipel-ide-core` implements it over
`Toolchains` (`toolchain_lsp.rs`): managed `rust-analyzer`/`taplo` start from their install;
managed jdtls runs the Equinox launcher directly on the resolved JDK (its `bin/jdtls` needs
Python). Missing-server notices carry `setup`, the UI shows **Set up**, which dispatches
`toolchain.setup` (component + missing requirements); when the setup finishes, missing
language servers restart.

### GUI walkthrough (Linux, Xvfb, 2026-10-10)

"Clean machine": `PATH` without `java`/`javac`/`jdtls`, `JAVA_HOME` unset, empty
`ARCHIPEL_TOOLCHAINS_DIR`. Java Gradle sample (`settings.gradle`, `build.gradle` with the
`java` plugin, `src/main/java/demo/App.java`).

1. Opening `App.java` raises *Java language server not found* with **Set up**
   (`toolchains/java-01-missing-server-setup.png`).
2. Clicking **Set up** (xdotool) starts `jdtls` and its requirement `jdk`; the status bar
   shows `Downloading jdtls 1.61.0… (+1)` with a progress bar
   (`toolchains/java-02-setup-progress.png`). `toolchain.jobs` at the same moment:
   `Downloading Eclipse JDT Language Server 1.61.0… 62%`, `Downloading JDK 25.0.4.1+1… 98%`
   (unpinned projects get the newest LTS JDK).
3. When both finish, jdtls restarts from the managed install on the managed JDK; typing
   `app.gr` + Ctrl+Space shows `greeting  App.greeting(String name) : String`
   (`toolchains/java-03-completion.png`). `toolchain.list` afterwards: `jdk` managed
   `…/jdk/25.0.4.1+1/bin/java`, `jdtls` managed `…/jdtls/1.61.0/bin/jdtls`.

### Headless end-to-end test

`cargo test -p archipel-ide-core --test toolchain_e2e -- --ignored --nocapture` repeats the flow
through commands with an empty `PATH`:

```text
setup in 12.6s: [{"component":"jdk","version":"25.0.4.1+1",...}]   # jdtls reused from ARCHIPEL_E2E_TOOLCHAINS
completion after 9.9s: ["clone", "equals", "finalize", "getClass", "greeting", ...]
servers: [("jdtls", "Ready")]
test result: ok. 1 passed
```

(A first run without a cached root installed both: `setup in 14.1s` for jdtls 1.61.0 + JDK.)

## Slice 2a — "missing toolchain" editor notification

`archipel_ide_toolchains::providers::project_requirements` detects what a project needs from
marker files (Gradle/Maven → `jdk` + `jdtls`, `Cargo.toml` → `rust-analyzer`, Bazel →
`bazelisk`). Core caches the missing components per language (refreshed on project open,
`toolchain.usePath` and finished installs); `toolchain.banner {path}` returns the bar for an
editor and `toolchain.dismissBanner {language}` hides it for the session. The bar uses the
reference `Banner.warningBackground`/`Banner.warningBorderColor` (dark) from
`docs/spec/data/themes.json`.

Walkthrough (same clean machine as above, language-server balloon dismissed first):

1. `toolchains/java-04-missing-toolchain-bar.png`: above `App.java`,
   *Java project: JDK and Eclipse JDT Language Server are missing* with **Set up** / **Dismiss**
   (the wording issue #75 asks for).
2. `toolchains/java-05-bar-setting-up.png`: after clicking **Set up**, the bar shows
   *Setting up…* with a spinner while both downloads run.
3. `toolchains/java-06-bar-done.png`: the bar is gone, the status line reads
   *JDK 25.0.4.1+1 installed* and jdtls is starting from the managed install.

Core test: `banner_lists_missing_components_for_the_project_language` (text, setup order,
other languages unaffected, `usePath` shrinking the list, dismiss).

## Slice 2b: one-click Java setup on a clean machine (Gradle, sync, completion)

User report on 0.0.3: Gradle was not set up and there was no completion. The chain now runs
end to end. It was verified on Xvfb (glow, 2026-10-10) with `PATH` stripped of `java`,
`javac`, `jdtls` and `gradle`, no `JAVA_HOME`, an empty `ARCHIPEL_TOOLCHAINS_DIR`, and a
scratch copy of `samples/java-gradle` with `gradlew`, `gradlew.bat` and `gradle/wrapper`
removed, so Gradle itself has to be installed:

1. `toolchains/java-07-clean-project-bar.png`: right after the project opens, with no editor
   yet, the bar reads *Java project: JDK, Gradle and the Java language server are missing*
   with **Set up** / **Dismiss**.
2. `toolchains/java-08-clean-setup-progress.png`: one **Set up** installs jdtls, the JDK and
   Gradle. The bar shows *Setting up…* and the status bar shows *Installing jdtls 1.61.0… (+2)*
   with a progress bar. Installed: Temurin 21.0.12.1+1, jdtls 1.61.0, Gradle 9.8.1 (from
   services.gradle.org, checked against the published `.sha256`).
3. `toolchains/java-09-clean-gradle-synced.png`: when the last install finishes, Gradle sync
   starts on its own with the managed Gradle and JDK (*sync finished, BUILD SUCCESSFUL*). The
   Gradle tool window lists the task groups, and the run widget has the new `java-gradle [run]`
   configuration selected (`java-gradle [test]` was created too).
4. `toolchains/java-10-clean-completion.png`: jdtls restarted from the managed install. After
   its import, typing `app.` opens the completion popup with `greeting(String name)`,
   `main(String[] args)` and the postfix templates.

With `gradlew` present, the bar lists only the JDK and the Java language server
(`detects_project_requirements_from_markers`); `gradlew` then gets the managed JDK through the
same task environment. That variant was not recorded on Xvfb.

## Pending (next slices)

- More catalog entries (rustup toolchains, Android SDK, kotlin-lsp, Node, Python, Go, ...);
  Rust (rust-analyzer) and Android SDK end-to-end runs.
- Settings > Languages & Frameworks > Toolchains page.
- Probing versions of `PATH` binaries; signature (GPG/minisign) verification.

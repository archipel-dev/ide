# Kotlin language server provisioning (#159)

Captured 2026-10-11 under Xvfb (1600x1000 virtual screen, `ARCHIPEL_RENDERER=glow`, debug build) with an empty
`$HOME`, driven through the control socket. Sample: a Gradle Kotlin DSL app (`kotlin("jvm")`, `fun main`).

| File | Result |
|---|---|
| `kotlin-lsp/159-kotlin-missing.png` | Opening `Main.kt` shows *Kotlin project: the Kotlin language server is missing* with **Set up**, plus the missing-server notice whose **Set up** maps to the `kotlin-lsp` component |
| `kotlin-lsp/159-kotlin-diagnostics.png` | After **Set up** (download, SHA-256 check, install, restart): `language.servers` reports `kotlin` `ready`, the run gutter icon is on `fun main`, and the type error shows in the editor, scrollbar and error counter |

Component: `kotlin-lsp` (JetBrains `kotlin-server` 263.6379.0). Source
`download.jetbrains.com/language-server/kotlin-server/<version>/kotlin-server-<version>.tar.gz`
(`-aarch64.tar.gz`, `.win.zip`), checksum from the `.sha256` beside it. The live download matched
`ab8ca4455dc2fc5fe1a24db2bccc46c104254d2c465155c4251ee65df8f3f7cc`. The archive bundles its runtime (no
JDK needed) and starts with `bin/intellij-server --stdio`. `download-cdn.jetbrains.com` answers 404 for
the archives, so the provider uses `download.jetbrains.com`, which redirects to a signed CDN URL.
Versions come from the `### v…` headings of the repository's `RELEASES.md`; the newest one whose
`.sha256` exists is used. JetBrains publishes macOS only as `.sit`, so macOS (and any failure to reach
the JetBrains host) falls back to fwcd/kotlin-language-server's `server.zip`, started with a JDK on `JAVA_HOME`.

Tests: provider test against a fake server (skips a version without archive, wrong request, fwcd fallback),
requirement detection test (Kotlin DSL, Kotlin plugin text, `src/main/kotlin`), resolver test
(`--stdio`), and the ignored real test `cargo test -p archipel-ide-lsp --test real_servers kotlin -- --ignored`
(completion on a Gradle sample, 5 s warm).

Notes: `toolchain.setup` takes explicit components (the banner and notice pass them); with `{}` it does nothing.
Maven Central returned HTTP 429 while Gradle resolved the Kotlin plugin; the build was retried with backoff until it passed.
Not covered: kotlin-lsp usages and inlays were not visible in the capture (the server was still importing the
Gradle project); they go through the same LSP code paths as Java.

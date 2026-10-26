# Editions roadmap

Goal: one Archipel IDE that covers what the JetBrains family covers per language: IntelliJ
IDEA (Java, Kotlin), RustRover (Rust), CLion (C, C++), WebStorm (JavaScript, TypeScript, web),
PyCharm (Python), GoLand (Go), and later Rider (C#), PhpStorm (PHP) and DataGrip (SQL and
databases). Archipel ships these as language packs in one product, not as separate IDEs.

Each pack has two halves:

- **Code intelligence** on the native pipeline in [code-intelligence-spec.md](code-intelligence-spec.md),
  raised tier by tier (G0 syntax, G1 locals, G2 project names, G3 types, G4 flow). The language
  server stays the fallback and the differential oracle until the native tier passes its
  conformance gate (spec 1.10).
- **Toolchain** integration: project model and build tool, run configurations, test runner, debugger,
  package manager, formatter and linters.

## Platform first

The packs build on shared work packages from the spec. Status:

| Package | Scope | Status |
|---|---|---|
| WP1 | Analysis core | merged (#36) |
| WP2 | Indexes, dumb mode, TODO, Local History | merged (#39) |
| WP3 | Daemon, inspections, intentions | merged (#40) |
| WP4 | G0 queries for all 20 grammars, typed editing, kit language service | in progress |
| WP5 | Resolve, usages, navigation, Search Everywhere | next |
| WP6 | Completion, quick doc, parameter info, formatter, templates, generate | next |
| WP7 | Refactoring framework, project undo | after WP5 |
| WP8 | LSP provider bridge and conformance harness | next |

## Language packs

| Pack (JetBrains counterpart) | Languages | Native tier target | Language server | Build / run / test | Debugger |
|---|---|---|---|---|---|
| IntelliJ IDEA | Java, Kotlin, Groovy | Java G3 then G4, Kotlin G2 | jdtls, Kotlin LSP | Gradle, Maven, JUnit 5, TestNG | JDWP via java-debug |
| RustRover | Rust | G2 | rust-analyzer | Cargo, `cargo test`, nextest | lldb / CodeLLDB (DAP) |
| CLion | C, C++ | G1 then G2 (includes) | clangd | CMake, Make, Meson, CTest, GoogleTest, Catch2 | lldb, gdb (DAP) |
| WebStorm | JavaScript, TypeScript, JSX/TSX, HTML, CSS | G2 (ES modules) | typescript-language-server, CSS/HTML servers | npm, pnpm, yarn, bun scripts, Jest, Vitest, ESLint, Prettier | Node inspector (js-debug DAP) |
| PyCharm | Python | G2 (imports) | pyright / basedpyright | venv, uv, poetry, pip, pytest, unittest, ruff | debugpy |
| GoLand | Go | G2 (packages) | gopls | go modules, `go test`, golangci-lint | delve |
| Data and markup | JSON, YAML, TOML, XML, properties, Markdown, Starlark | full native (key paths) | taplo, yaml, json, lemminx | Bazel, Gradle and Maven files | n/a |
| DataGrip (later) | SQL | G2 (schema) | sqls | database connections (mongo-raabit pattern) | n/a |
| Rider, PhpStorm (later) | C#, PHP | G1 | OmniSharp/csharp-ls, intelephense | dotnet, composer, phpunit | netcoredbg, xdebug |

Existing building blocks in the repo: 20 tree-sitter grammars, the LSP registry, `archipel-ide-dap`
(debug adapter client), `archipel-ide-runnables`, `archipel-ide-toolchains` (downloads with SHA-256),
`archipel-ide-gradle`, `archipel-ide-bazel`, `archipel-ide-android`, and the `rust` plugin.

## Order

1. Platform: WP4 (all languages to G0), then WP5, WP6 and WP8 in parallel, then WP7.
2. Data and markup pack (exercises the whole pipeline natively, needed by every build tool).
3. Java/Kotlin and Rust packs in parallel (largest parity weight; Rust is the IDE's own language).
4. C/C++ and JavaScript/TypeScript packs.
5. Python and Go packs.
6. SQL/database, then C# and PHP.

Each pack ends with: conformance corpus and differential gate against its language server,
toolchain detection and setup through `archipel-ide-toolchains`, run/test/debug configurations,
parity rows updated with measured evidence, and scenes with screenshots.

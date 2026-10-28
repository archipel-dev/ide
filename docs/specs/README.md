# Clean-room specifications

Archipel IDE follows IntelliJ IDEA's behavior and architecture without using its code. The
specifications under this directory are the only channel between the two sides of a
two-team clean-room process:

| Team | Reads | Writes |
|---|---|---|
| **Specification** | IntelliJ IDEA Community source (Apache-2.0), JetBrains documentation, observed behavior | Markdown in `docs/specs/` only |
| **Implementation** | `docs/specs/`, the rest of `docs/`, LSP and language specifications (JLS, ...), observed behavior | Rust code, tests, scenarios |

## Rules

- **Specs describe, they do not transcribe.**
  - Allowed: concepts, data models as fields and meanings, contracts, algorithms as prose or numbered steps, enumerated rules and edge cases, and observable behavior (ordering, priorities, timings).
  - Allowed: user-visible message texts. They are short facts.
  - Forbidden: no code from JetBrains, no copied comments, no line-by-line paraphrases.
  - Examples in specs are small snippets the spec author wrote.
- **Specs cite their sources** as `ic: <path in intellij-community>` so reviewers can audit them. The reference is a path only, never the code.
- **Implementers never open JetBrains source.** That covers the intellij-community repository, decompiled jars and IDE internals. When a spec is unclear, the implementer asks the specification team for a clarification in the spec.
- **Every requirement has a stable ID** (`JAVA-IMP-003`, `CI-ALT-007`, `PLAT-PSI-014`), a priority (`MUST`/`SHOULD`/`LATER`) and an acceptance criterion. Code, tests and scenarios reference these IDs.
- **Reviewers check both directions:**
  - a spec PR must contain no code from JetBrains;
  - an implementation PR must cite the spec IDs it implements and must not depend on knowledge that is not in the specs.

## Index

| Area | Directory | Covers |
|---|---|---|
| Platform | `platform/` | VFS, PSI, stubs and indexes, dumb mode, caching, concurrency, scopes, language extension points, saving and autosave |
| Code insight | `codeinsight/` | highlighting daemon, inspections, quick fixes and Alt+Enter, auto-import, completion framework, navigation |
| Java | `java/` | stubs, resolve, types and inference, error catalog, imports, completion, inspections, Spring notes |
| Inventory | `inventory/` | feature inventory of IntelliJ IDEA Community (actions, settings, tool windows, inspections, intentions, file types) with spec coverage and ranked gaps; facts only, no requirements |
| Terminal UI | `tui/` | the terminal frontend mirrors the GUI: cell-by-cell chrome, key fallbacks, shared UI layer (prefix `TUI`); a binding product decision, not an IntelliJ observation |

## Coverage

`cargo run -p xtask -- spec-coverage` reports how many requirements are implemented
(`docs/specs/coverage.md`, generated). Reference requirement IDs from code and from tests,
scenarios or scenes; pre-spec code is credited with evidence in `docs/specs/coverage.toml`.

# Daemon, inspections and intentions (`archipel-ide-daemon`)

Part of the analysis pipeline (see [architecture.md](architecture.md)). The daemon turns snapshots
into highlights; quick fixes and intentions turn highlights into edits.

## Passes and scheduling

`Daemon::tick(host, events)` runs once per frame (from `IdeState::poll_analysis`). Passes
(`Syntax`, `General` with `Annotator`s, `LocalInspections`, `ExternalAnnotators`,
`IdentifierHighlighter`, `LineMarkers`) run as `Daemon`-priority host jobs on a snapshot;
`Syntax` runs inline. The pure `Scheduler` (injected clock) enforces: debounce 300 ms (setting
`daemon.autoReparseDelayMs`), visible range first then the whole file, prerequisites
(`Pass::after`), and cancel-on-edit (a new file revision cancels the file's running passes; a
stale result is mapped through the edit journal or dropped). Highlights live in
`FileMarkup`: per-owner slices replaced atomically, identical infos recycled (stable
`HighlightId`), ranges shifted through edits, budgets (1000 per tool, 2000 per file).

## Inspections

Tier-A inspections need no resolve and are driven by node-kind tables
(`inspection/kinds.rs`): EmptyCatchBlock, EmptyStatementBody, UnreachableCode,
AssignmentInCondition, ConstantCondition, RedundantParentheses, SelfAssignment,
DuplicateBranches, TodoComment, DuplicateKey (JSON, YAML, TOML, properties, XML),
TrailingWhitespace, MixedIndentation, LongLine. Not yet: unused local/import and shadowed
variable (need G1 locals), typo (needs a dictionary).

Profiles are TOML: `<project>/.archipel/inspection-profiles/<name>.toml` and
`<config>/inspection-profiles/<name>.toml` (project shadows application); the active one is the
setting `inspections.profile` (default `Project Default`, which is the built-in `Default`
until a file exists).

```toml
base = "Default"
[tools.EmptyCatchBlock]
enabled = true
level = "error"        # error | warning | weak_warning | information | text_attributes
[[tools.TodoComment.scopes]]
scope = "tests"        # all | production | tests | generated | <glob>
enabled = false
[tools.LongLine.options]
max = 140
```

Suppression: a comment `noinspection <Id>[, <Id>]` (or `ALL`) directly above a statement or
declaration, or trailing on its first line; `noinspection-file <Id>` in the file header;
Java `@SuppressWarnings("Id")`; Rust `#[allow(archipel::Id)]` (and mapped rustc lint names).
The Alt+Enter submenu of an inspection generates these comments.

## Alt+Enter

`assemble` builds the rows: fixes on the caret line (Error 30, Inspection 20, remote-error 10),
intentions (0), actions of other sources such as the LSP list (deduplicated by family and
text, native first), sorted by group priority plus priority weight (top +20, high +3, low -3,
bottom -20) descending, then by text; cached per file revision, caret and markup version.
Choosing a row returns a pure `ModCommand`; core applies `UpdateFile` as one undo step per
document with an optimistic revision check.

## Seams used

`analysis_modules.rs` (`daemon` entry: services + `inspection.*` commands),
`analysis_host.rs` (one line: `poll_daemon` after the host's housekeeping) and `lib.rs`
(module declaration). The daemon itself is a registry service (`DaemonService`).

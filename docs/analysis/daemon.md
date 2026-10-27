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

## Language-server diagnostics (external pass)

The daemon is the only producer of editor markup. A language server's diagnostics enter as the
`ExternalDiagnostics` pass, IntelliJ's `ExternalAnnotator` contract in three steps:

1. *collect*: core mirrors what `LspManager` published into `Shared::external` (the
   `ExternalDiagnostics` feed, keyed by project-relative path; positions already resolved to byte
   ranges of the document revision the editor had) and calls `Daemon::diagnostics_published(file)`
   (`external_diagnostics.rs`, after every `poll_lsp`). While jdtls imports the build
   (`language/status` between `Starting` and `ServiceReady`) its diagnostics are held back; the
   transition to `ServiceReady` triggers a publish of everything.
2. *run*: the pass reads the feed entry on a snapshot (it listens to `DiagnosticsPublished` only; edits
   shift the markup through the journal until the server republishes).
3. *apply*: the result, mapped from the entry's revision to the current one, replaces the
   `lsp:<server>` slice of the file's markup.

`FileMarkup::reconcile` then deduplicates native and `lsp:*` entries (spec 1.10): same range and kind is
one problem, the native entry wins when the language graduated (`CompletenessTier >= G2`), the
server's otherwise; native syntax errors give way once a server has published for the file unless
the language graduated. The loser stays stored but *shadowed*: queries, iterators and counters
skip it, so the Problems view, the error stripe, F2/⇧F2 and the squiggles read one result
(`problems.rs` only converts markup to lines and columns; files that are not open come from the feed).

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
LSP code actions are attached to the server's highlight they fix (the diagnostic they name, else the one
at the caret): a quick fix of an error at the caret joins the Error group, of a lesser problem the
Inspection group, of a problem elsewhere on the line RemoteError; refactor, source and quick-assist
actions are intentions. Choosing one applies its `WorkspaceEdit` (one undo step per document) or
executes its command (`java.apply.workspaceEdit` is applied by the editor, others go to the server).
Choosing a row returns a pure `ModCommand`; core applies `UpdateFile` as one undo step per
document with an optimistic revision check.

## Seams used

`analysis_modules.rs` (`daemon` entry: services + `inspection.*` commands),
`analysis_host.rs` (one line: `poll_daemon` after the host's housekeeping) and `lib.rs`
(module declaration). The daemon itself is a registry service (`DaemonService`).

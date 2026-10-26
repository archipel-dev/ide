# Agents mode backend

`archipel-ide-agents` runs Claude Code and Codex as structured-protocol child processes, one git
worktree per session, and feeds the Agents mode UI through `archipel_ide_core::agents::AgentsBackend`
(`RealAgentsBackend` in `agents_real.rs`). The scripted `MockAgentsBackend` stays for
`ARCHIPEL_AGENTS_DEMO=1`, the `agents.demo` command and tests. Product decisions are in
`agents-mode.md` (design), CLI facts and their sources in [agent-integrations.md](agent-integrations.md).

## Evidence levels

`agent-integrations.md` documents flags and resume behaviour but **not** the message shapes of
`claude --output-format stream-json` or `codex exec --json`. The adapters therefore use:

- documented: the flags (`-p`, `--output-format`, `--input-format`, `--verbose`,
  `--permission-mode`, `--session-id`, `--resume`; `codex exec`, `exec resume`, `-c`, `sandbox_mode`,
  `approval_policy`);
- inferred (marked `[INFER]` in the adapter module docs, unverified until a real install runs them):
  every JSON shape, `--permission-prompt-tool stdio` with `control_request`/`control_response`,
  `--json` placement before `resume`, the `--` before the Codex prompt.

All parsing is defensive: unknown fields are ignored, unknown tools and item types become a generic
`Other` tool row, unknown event types an `Event <type>` row, non-JSON lines are dropped.

## Lifecycle

1. `launch` creates the session at once (state Working) and returns; a worker thread creates the
   worktree (`<data dir>/archipel/worktrees/<repo>-<hash6>/<slug>`, or the configured base
   directory) on a new `archipel/<slug>` branch from the checked-out branch, then spawns the CLI.
2. Claude runs one long-lived process (`--session-id <uuid>` pre-assigned, prompts and follow-ups
   as stream-json lines on stdin, permission answers as `control_response`). Codex runs one
   `codex exec --json` process per turn and resumes the thread with `codex exec resume`; follow-ups
   sent while it works are queued. Codex never asks for approval, the sandbox decides.
3. Changes are the worktree against the merge base with the base branch (`diff_stat`, untracked
   files, `FileDiff` per file), computed off-thread after turns and file-writing tools.
4. Merge: commit everything in the worktree, then `git merge --squash` (or `--no-ff`) into the base
   branch of the project checkout (must be checked out and clean), then remove the worktree and
   branch. Conflicts restore the checkout. Discard removes the worktree and branch.
5. Persistence: `<data dir>/archipel/agents/<repo>-<hash6>/sessions/<id>/{meta.json,transcript.jsonl}`.
   The log holds the transcript updates, passed through a redactor (environment secret values, token
   shapes, `NAME_KEY=value`). On IDE start, sessions that were running become Stopped; a message
   resumes the same agent session (`--resume <id>` / `exec resume <thread>`).

## Guardrails

Max parallel agents (setting `agents.maxParallel`, also counts spawned agents), spawn depth
(`EngineConfig::max_spawn_depth`, default 2, enforced by `Engine::launch_child`), environment
allowlist plus the agent's own variable prefixes, argument vectors without a shell, own process
group so Stop reaches the tools an agent started, graceful signal then kill after
`stop_grace`, idle agent processes released after ten minutes.

## Tests

`crates/archipel-ide-agents/tests/e2e.rs` runs the real engine against a temporary git repository
with `fake-agent` (`tests/support/fake_agent.rs`), which replays the fixtures in `tests/fixtures/`
and echoes stdin. The fixtures are written from the schemas above, not recorded from a real CLI.

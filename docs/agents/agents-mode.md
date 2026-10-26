# Agents mode

Agents mode runs several AI coding agents in parallel, each in its own git worktree, and lets
the user review and merge their work without leaving the IDE. It is one window with two modes
over the same project (`Code | Agents`), not a second app. Behaviour is modeled on Orca
(<https://github.com/stablyai/orca>, MIT; see `THIRD_PARTY_NOTICES.md`); the UI is built only
from Archipel Kit components and stays close to the IntelliJ design.

Status: the UI ships with a mocked backend (`ARCHIPEL_AGENTS_DEMO=1` or the `agents.demo`
command). Git worktree management (`archipel_ide_git::worktrees`) is real. Agent CLI facts are
in [agent-integrations.md](agent-integrations.md).

## Decisions

These were decided with the product owner and supersede earlier drafts.

1. **Structured agents only, no terminal.** Agents mode never shows a terminal. A session is a
   native transcript built from typed events. Supported agents are those with a structured
   protocol: Claude Code (`--output-format stream-json`, follow-ups via
   `--input-format stream-json`) and Codex (`codex exec --json` / app-server JSON-RPC).
   Gemini CLI and OpenCode follow once their structured modes are verified. Agents without one
   (for example aider) are not offered.
2. **Threads as tabs plus an inspector.** Each agent session is a thread that opens as an
   editor-style tab (JBTabs). A right-hand inspector follows the active tab: status, agent and
   model, worktree and branch, elapsed time, tokens, changed files, sub-threads, a pending
   question, and Review / Open in Code / Merge.
3. **One composer.** A single composer at the bottom of the active thread, in the style of the
   Claude Code desktop app: auto-growing text area, Enter sends, Shift+Enter adds a line, `@`
   attaches files, `/` opens commands, chips for agent/model and permission mode, and a send
   button that turns into Stop while the agent works. New threads open from "+" on the tab row,
   "+" in the Agents tree, or `agents.newThread`; only a new thread may fan out to several
   agents (one tab each). There is no global prompt bar and no second reply field.
4. **Permission questions** appear once, as a card in the transcript (Allow once / Always allow
   / Deny). The composer only hints at them.
5. **Agents may drive the IDE and spawn agents.** Through a scoped, token-gated control socket
   agents can open files, run tests and query inspections, and launch further agents.
   Guardrails: the global max-parallel-agents setting also covers spawned agents, spawn depth
   is limited (default 2), children appear under their parent in the tree, and every spawn
   raises a notification. A session can only act inside its own project.
6. **IDE close stops agents.** Running agents are stopped on exit and can be resumed with their
   transcript replayed. A background host that keeps agents alive is a later, opt-in step.
7. **Worktrees live outside the repository**, under the data dir
   (`<data_dir>/archipel/worktrees/<repo>-<hash6>/<slug>`). Agent hooks are injected per launch
   only; the user's global agent configuration is never modified.
8. **Visible in Code mode.** A status-bar widget (`2 running · ? 1 input`), a compact Agents
   tool window and balloons (Reply / Review) keep running agents in view.

## Architecture

- `archipel_ide_core::agents`: the session model, `AgentEvent` transcript (UserPrompt,
  AssistantText, Thinking, ToolCall, PermissionRequest, SubagentSpawned, TurnResult, Error) and
  the `AgentsBackend` trait the UI talks to. `agents_mock.rs` implements it with scripted
  sessions; real adapters implement the same trait.
- `archipel_ide_git::worktrees`: list/add/remove/prune/lock, lineage, safety rules ported from
  Orca, include files and shared directories.
- `archipel_ide_ui::agents_ui`: mode switch, Agents tree, thread tabs, transcript, composer,
  inspector, review screen and the Code-mode widgets. Every icon goes through one
  `glyph(Concept)` table so kit glyphs (archipel-dev/kit#51) swap in one line each.
- Opening an agent's file in Code mode needs a narrow re-root path: `LspManager::set_project`
  restarts every language server and `install_project` drops all terminals, so switching to a
  worktree must not reuse them.

## Next work packages

1. **Protocol adapters** for Claude Code and Codex: spawn with stdio JSON, map events to
   `AgentEvent`, permission round-trips, resume. Replaces the mock behind `AgentsBackend`.
2. **Session lifecycle**: worktree per session, persistence of transcripts, resume after
   restart, notifications and unread state.
3. **Review and merge for real**: diff against the merge base, squash or merge, discard and
   cleanup, inspections on agent diffs (daemon), comments on diff lines sent back as a prompt.
4. **Scoped control socket and CLI** for agents calling back into the IDE, with the guardrails
   from decision 5.
5. **Kit pieces**: a multiline text field, a round icon button and a transcript/chat list
   component, replacing the prototype's workarounds.
6. **TUI**: the same mode in `archipel-kit-tui` (NvChad look).

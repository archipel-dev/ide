# Agent hook fixtures

These files are **synthetic**. They were written by hand from the documented schemas; none was
recorded from a running agent, and no real session, path, prompt or secret appears in them.
Paths, ids and prompts are invented (`/data/archipel/worktrees/ide/fix-login`, a UUID session id).
Treat them as shape contracts for the detector tests, not as captures. Replace or extend them with
recorded streams once a real install has been checked (see "Verification status" in
`docs/agents/agent-integrations.md`).

| Directory | Source of the schema |
|---|---|
| `claude/` | Claude Code hooks reference, https://code.claude.com/docs/en/hooks (common fields, per-event fields, `Stop`, `StopFailure`, `SessionEnd`, `Notification`, `PermissionRequest`, `PostToolUse` examples). `session-settings.json` is the per-launch `--settings` file Archipel would generate (schema: same page, "settings.json hooks schema"). |
| `codex/` | OpenAI Codex hooks page, https://learn.chatgpt.com/docs/hooks (formerly https://developers.openai.com/codex/hooks): common fields `session_id`, `transcript_path`, `cwd`, `hook_event_name`, `model`; per-event `turn_id`, `tool_name`, `tool_input`, `tool_response`, `prompt`, `stop_hook_active`, `last_assistant_message`. Field values for `transcript_path`, `tool_use_id` and `tool_response` are invented; the docs give no full example for them. |
| `gemini/` | Gemini CLI hooks reference, https://geminicli.com/docs/hooks/reference/ (common fields incl. `timestamp`; `BeforeAgent`, `AfterAgent`, `BeforeTool`, `AfterTool`, `Notification`, `SessionStart`, `SessionEnd`). The inner shape of `tool_response` and `Notification.details` is invented. |
| `opencode/` | OpenCode has no stdin hooks; events reach a plugin as `{type, properties}`. Event names are from https://opencode.ai/docs/plugins/ . The property shapes (`sessionID`, `status.type`, `permission`, `patterns`, `metadata`, `requestID`) are not documented there; they follow what the MIT-licensed Orca source reads from live OpenCode 1.18 captures (`src/shared/opencode-permission-status.test.ts`, `src/main/opencode2/status-plugin-setup-source.ts`). Lowest confidence of all fixtures. |
| `osc7501/` | Program Status Protocol v0.4, https://www.superlogical.com/rex/docs/build/program-status . `reports.ndjson` holds one escaped byte string per line with the expected parse. |
| (none) aider | Aider has no hooks and no machine payload. Its only per-launch signal is `--notifications-command`, which runs a command with no documented stdin or arguments, so there is nothing to fixture. |

Secrets: none of these files contain tokens or real paths. Do not paste recorded transcripts here
without scrubbing `transcript_path`, `prompt`, `tool_input` and `last_assistant_message`.

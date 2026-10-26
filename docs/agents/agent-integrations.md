# Agent CLI integration facts

Status: Wave 0 spike (WP-0A), **documentation only**. No agent CLI was installed or run. Every fact
carries its source:

- `[DOC url]` stated on an official documentation page, read on 2026-10-26.
- `[ORCA path]` read in the MIT-licensed Orca source (`stablyai/orca`, paths relative to its repo root). Orca is
  evidence of what live captures looked like, not a specification. Nothing was run from it.
- `[INFER]` our reading of the above; not stated anywhere. Each of these must be confirmed on a real
  install before a profile default depends on it (see "Verification status" at the end).

Synthetic payloads built from these schemas live in `crates/archipel-ide-agents/tests/fixtures/`.

## Summary

| | Claude Code | Codex CLI | Gemini CLI | OpenCode | Aider |
|---|---|---|---|---|---|
| Command | `claude` | `codex` | `gemini` | `opencode` | `aider` |
| Per-launch hook injection | `--settings <file or inline JSON>` (command hooks) | `-c` TOML overrides plus `--dangerously-bypass-hook-trust` [INFER, see 2]; fallback `-c notify=[...]` | `GEMINI_CLI_SYSTEM_SETTINGS_PATH=<file>` [INFER: merge with user hooks undocumented] | `OPENCODE_CONFIG_DIR=<overlay>` with `plugins/archipel.js` | none; `--notifications` (BEL) or `--notifications-command` |
| Hook events used | `SessionStart UserPromptSubmit PreToolUse PermissionRequest PostToolUse Notification Stop StopFailure SessionEnd` | `SessionStart UserPromptSubmit PreToolUse PermissionRequest PostToolUse Stop Interrupt SessionEnd` | `SessionStart BeforeAgent BeforeTool AfterTool Notification AfterAgent SessionEnd` | plugin events `session.status` `session.idle` `permission.asked` `permission.replied` `session.error` | n/a |
| Initial prompt via argv | `claude "text"` (positional, stays interactive) | `codex "text"` (positional, stays interactive) | `gemini -i "text"` (`-p` is headless) | `opencode --prompt "text"` | no: `--message` runs once and exits; paste after ready |
| Resume last | `claude --continue` | `codex resume --last` | `gemini --resume latest` | `opencode --continue` | `aider --restore-chat-history` (history file only) |
| Resume by id | `claude --resume <id\|name>`; ids can be pre-assigned with `--session-id <uuid>` | `codex resume <SESSION_ID>` | `gemini --resume <index>` (no id form documented) | `opencode --session <id>` (`--fork` to branch) | none |
| Approval flag | `--permission-mode <mode>` / `--dangerously-skip-permissions` | `-a on-request\|never`, `-s <sandbox>`, `--yolo` | `--approval-mode default\|auto_edit\|plan\|yolo` | config `permission` (`allow\|ask\|deny`), `--auto` | `--yes-always` |
| Title / status signal | title glyph `✳` idle, spinner working; OSC 7501 when the terminal answers the query (v2.1.295+) | title items (`spinner`, `project`), OSC 9 or BEL notifications | glyphs `◇` idle, `✦` working, `⏲` silent working, `✋` permission | title `OC \| <session>` | BEL when ready (`--notifications`) |
| First-run dialog | workspace trust (per repo root, worktrees share the main checkout's trust) | trust prompt per project (`projects.<path>.trust_level`) | folder trust: Trust folder / Trust parent / Don't trust | none documented | none documented |

## 1. Claude Code (`claude`)

**Per-launch hooks.** `--settings` takes "a path to a settings JSON file or an inline JSON string"; its values "override the same keys in
your `settings.json` files for this session. Keys you omit keep their file-based values"; the file must be a regular file of at most 2 MiB
[DOC https://code.claude.com/docs/en/cli-reference]. Precedence: managed > command line > project local > project > user; list keys such as
`permissions.allow` are combined across files [DOC https://code.claude.com/docs/en/settings]. `--settings` can set any key a user settings
file can, and "lasts one session and doesn't write to any file" [DOC same]. So Archipel writes
`<data dir>/agents/<repo-key>/sessions/<id>/claude-settings.json` and launches `claude --settings <that file>`; the user's `~/.claude` is untouched.
Template: `tests/fixtures/claude/session-settings.json`.
[INFER] Whether `hooks` arrays in `--settings` are appended to or replace the user's hooks is not stated on the pages read (only list
keys like `permissions.allow` are documented as merging). Check on a real install that a user's own hook still fires.
Related flags: `--setting-sources user,project,local` restricts which files load; `--bare` skips hooks, skills, MCP and CLAUDE.md
(do not use for interactive agents) [DOC cli-reference]. `CLAUDE_CONFIG_DIR` moves settings, history and plugins [DOC settings]; not needed.

**Hook command contract.** Each hook is `{ "type": "command", "command": "...", "timeout": <seconds> }`, grouped event -> matcher group
(`matcher`, optional) -> `hooks`; `timeout` default 600 s for command hooks; exit 0 parses stdout as JSON if it is a single object, exit 2 blocks
(stderr is the reason), any other code is non-blocking; `args` runs the command without a shell (exec form) [DOC https://code.claude.com/docs/en/hooks].
Our bridge must always exit 0 and print nothing (or `{}`), and keep a short timeout so a dead IDE never stalls the agent.

**Events and fields.** Event names: `SessionStart, Setup, UserPromptSubmit, UserPromptExpansion, PreToolUse, PermissionRequest, PermissionDenied,
PostToolUse, PostToolUseFailure, PostToolBatch, Notification, MessageDisplay, SubagentStart, SubagentStop, TaskCreated, TaskCompleted, Stop,
StopFailure, TeammateIdle, InstructionsLoaded, ConfigChange, CwdChanged, DirectoryAdded, FileChanged, WorktreeCreate, WorktreeRemove, PreCompact,
PostCompact, PreModelSwitch, PostModelSwitch, Elicitation, ElicitationResult, SessionEnd` [DOC hooks]. Common stdin fields: `session_id`,
`prompt_id`, `transcript_path`, `cwd`, `scratchpad_dir`, `permission_mode` (`default|plan|acceptEdits|auto|dontAsk|bypassPermissions`),
`effort.level`, `hook_event_name`, plus `agent_id`/`agent_type` inside a subagent [DOC hooks]. Per event [DOC hooks]:

| Event | Extra fields | Maps to (design 7.2) |
|---|---|---|
| `SessionStart` | `source` (`startup\|resume\|clear\|compact\|fork`), `model` (may be absent) | `Starting -> Idle`, store `session_id` as `provider_session_id` |
| `UserPromptSubmit` | `prompt` | `PromptSubmitted` -> `Working` |
| `PreToolUse` | `tool_name`, `tool_input`, `tool_use_id` | `Working`, tool preview; `AskUserQuestion` here means `NeedsInput(Question)` [ORCA src/shared/agent-hook-listener/providers/claude-events.ts] |
| `PermissionRequest` | `tool_name`, `tool_input`, optional `permission_suggestions` (no `tool_use_id`) | `NeedsInput(Permission)` |
| `PostToolUse` / `PostToolUseFailure` | `tool_name`, `tool_input`, `tool_response`, `tool_use_id`, optional `duration_ms` | resolves a prompt; `Working` |
| `Notification` | `message`, `title`, `notification_type` (`permission_prompt`, `idle_prompt`, `auth_success`, `elicitation_dialog`, ...) | secondary signal; `auth` kind candidate |
| `Elicitation` / `ElicitationResult` | MCP dialog | `NeedsInput(Question)` / restore previous state [ORCA claude-events.ts] |
| `Stop` | `stop_hook_active`, `last_assistant_message`, `background_tasks[]`, `session_crons[]` | `TurnFinished` (not for user interrupts) |
| `StopFailure` | `error` (`rate_limit`, `authentication_failed`, `billing_error`, `server_error`, `unknown`, ...), `error_details`, `last_assistant_message` (the API error text) | `Errored` |
| `SessionEnd` | `reason` (`clear\|resume\|logout\|prompt_input_exit\|other`) | `Exited` hint |

"Stop does not run if the stoppage occurred due to a user interrupt" [DOC hooks], so an interrupt produces no hook: the detector needs
the screen or OSC channel to leave `Working` after Esc. Orca infers cancellation from `is_interrupt` and other evidence
[ORCA claude-events.ts]; treat as [INFER]. `Stop` with a non-empty `background_tasks` or `session_crons` means "paused", not done; the Orca
roster code holds the lead open the same way [ORCA claude-events.ts, claude-pane-hold-evidence.ts]. Fixtures: `tests/fixtures/claude/*.json`.

**Resume.** `claude --continue` / `-c` loads the most recent conversation in the current directory (skips `-p`/SDK sessions);
`claude --resume <id|name>` / `-r` resumes by id, name or `.jsonl` path and searches the current project and its git worktrees, then every project;
`--session-id <uuid>` pre-assigns the id (must be a valid UUID); `--fork-session` branches when resuming; `--name`/`-n` sets the display name
[DOC cli-reference]. Because `--session-id` exists, Archipel can generate the UUID at launch and never has to wait for `SessionStart`
to learn it, then resume with `claude --resume <uuid>` exactly.

**Initial prompt.** `claude "query"` starts an interactive session with that prompt; `-p` is headless [DOC cli-reference]. Orca additionally uses
`--prefill` to place a draft without submitting [ORCA src/shared/tui-agent-config.ts, claude entry]; `--prefill` does **not** appear in the CLI reference, so
treat it as undocumented and do not depend on it. Pastes over 800 characters or three lines are collapsed to `[Pasted text #N +L lines]` but "still sends the full
content" [DOC https://code.claude.com/docs/en/terminal-config]; Enter submits, Ctrl+J inserts a newline, Shift+Enter needs the kitty keyboard protocol
(not available in JetBrains-style terminals) [DOC same]. Orca sends Ctrl+Enter as CSI-u for Claude [ORCA tui-agent-config.ts `ctrlEnterEncoding: 'csi-u'`].

**Approval.** `--permission-mode` accepts `default`, `acceptEdits`, `plan`, `auto`, `dontAsk`, `bypassPermissions`, or `manual` (alias of `default`, v2.1.200+);
`--dangerously-skip-permissions` equals `bypassPermissions`; `--allow-dangerously-skip-permissions` only adds it to the Shift+Tab cycle
[DOC cli-reference]. `permissions.defaultMode` values `auto` and `bypassPermissions` "don't take effect from project or local settings", so pass the flag per launch
[DOC settings]. Profile: `yolo_args = ["--dangerously-skip-permissions"]`, never default (as designed).

**Title, OSC, bell.** Claude sets the terminal title; `CLAUDE_CODE_DISABLE_TERMINAL_TITLE=1` disables automatic title updates and also turns off OSC 7501 status
reporting [DOC https://code.claude.com/docs/en/env-vars]. Rest glyph is `✳` (ASCII fallback `* `), working shows a spinner (braille `U+2800-28FF`, and since 2.1.228 quarter circles
`U+25D0-25D3`) [ORCA src/main/runtime/agent-state-rules/claude.json, src/shared/agent-title-glyphs.ts]. **OSC 7501 is emitted by Claude Code itself** from v2.1.295: "If your terminal
implements the OSC 7501 Program Status Protocol, it can show whether each interactive Claude Code session is working, waiting on you, or done"; with support detected, the title
carries no status glyph; not emitted inside tmux or for background sessions [DOC terminal-config, "See session status in your terminal"]. The protocol: `ESC ] 7501 ; key=value:key=value ST`,
states `idle|working|done|blocked|error|clear`, kinds `permission|question|auth`, base64 `msg`/`title`, whole sequence at most 4096 bytes; a program probes with `ESC ] 7501 ; ? ST` and the terminal answers
`ESC ] 7501 ; ? [:states=...:kinds=...] ST` [DOC https://www.superlogical.com/rex/docs/build/program-status; ORCA src/shared/program-status/protocol.ts]. The spec page lists no emitters; the Claude docs page is the only
evidence that Claude emits it. Consequence for the design: **answering the query flips Claude from title glyphs to OSC 7501**. Do both only by deliberate choice: the detector must handle both channels, and the
query reply must be written only after the OSC parser exists (otherwise Claude drops the glyph and nobody listens). Bell/notifications: desktop notification only in Ghostty, Kitty and iTerm2 by default;
`preferredNotifChannel: "terminal_bell"` rings BEL [DOC terminal-config]. Fixtures: `tests/fixtures/osc7501/reports.ndjson`.

**Trust dialog.** "Claude Code shows the trust dialog in interactive sessions only" (never for `-p`/SDK). In a repository trust is keyed on the git repository root; "In a worktree, it uses the main
checkout's root", so a worktree of an already trusted repo should not prompt again; outside a repo it is keyed on the starting directory; in the home directory it is session-only
[DOC https://code.claude.com/docs/en/permissions, "Project allow rules and workspace trust"]. Screen text (live dialog): a choice row `1. Yes, I trust this folder` and the footer
`Enter to confirm · Esc to cancel` [ORCA src/main/runtime/trust-dialog-status-observer.ts]. The prompt delivery gate must treat that screen as `NeedsInput(Trust)` and never paste into it (design 6.5).
We do not pre-write trust (decision 12.5).

## 2. Codex CLI (`codex`)

Docs moved: `developers.openai.com/codex/*` redirects to `https://learn.chatgpt.com/docs/*`.

**Per-launch hooks.** Hooks load from `hooks.json` or an inline `[hooks]` table in `config.toml` "next to each active config layer" (`~/.codex`, `<repo>/.codex`); hooks from all matching
sources run, higher layers do not replace lower ones; project-local hooks load only when the project `.codex/` layer is trusted; `[features] hooks = false` disables them
[DOC https://learn.chatgpt.com/docs/hooks]. **Non-managed hooks must be reviewed and trusted before running; trust is tied to the hook's hash, so new or changed hooks are skipped until re-trusted**;
the one documented per-invocation switch is `--dangerously-bypass-hook-trust`, "runs enabled hooks without persisted trust for that invocation" [DOC hooks]. Trust is persisted as
`[hooks.state.<key>] trusted_hash = "<sha>"` tables in `config.toml` [ORCA src/main/codex/config-toml-hook-trust-edit.ts], which is exactly the global edit we avoid.
`-c, --config key=value` overrides any config value, "Values parse as TOML if possible" [DOC https://learn.chatgpt.com/docs/developer-commands?surface=cli]; the inline `[hooks]` table "uses the same event schema as `hooks.json`"
and `features.hooks` enables it [DOC https://learn.chatgpt.com/docs/config-file/config-reference.md]. The docs do not say that hooks can be passed via `-c` or an environment variable ("I can't confirm either exists").
Candidate per-launch recipe **[INFER, must be proven on a real install]**:

```
codex -c features.hooks=true \
      -c 'hooks.Stop=[{hooks=[{type="command",command="archipel-cli agent hook --source codex",timeout=5}]}]' \
      ... one -c per event ... \
      --dangerously-bypass-hook-trust "<prompt>"
```

Fallbacks, in order: (a) `-c 'notify=["archipel-cli","agent","hook","--source","codex"]'`: `notify` is an `array<string>` command that "Codex passes a JSON payload"; a secondary (non-official) source says it fires only for
`agent-turn-complete`; `notify` is honoured at user level, ignored in project `.codex/config.toml` [DOC config-reference; secondary https://codex.danielvaughan.com/2026/04/13/codex-cli-notification-pipeline-osc9-hooks-alerts/].
That gives turn-finished only, so the screen/title channels carry working and blocked. (b) `-c tui.notifications=true -c tui.notification_method=osc9` (`auto|osc9|bel`) for a terminal-side signal [DOC config-reference].
(c) A per-session `CODEX_HOME` overlay is **not recommended**: `auth.json`, `config.toml`, the sessions directory and `resume` all live under it [DOC config-reference mentions `$CODEX_HOME/auth.json`, logs, profile files], so an overlay needs copies of credentials and breaks
`codex resume`. (d) Writing `<worktree>/.codex/hooks.json` works only when the project layer is trusted and then pollutes the worktree; avoid.
Orca's own list of Codex flags that disable its shared-daemon reuse includes `--dangerously-bypass-hook-trust`, `-c`, `--enable`, `--disable`, `--profile`, confirming these exist in the CLI [ORCA src/shared/codex-shared-server-command.ts].

**Events and fields.** Events: `SessionStart, SessionEnd, PreToolUse, PermissionRequest, PostToolUse, PreCompact, PostCompact, UserPromptSubmit, SubagentStart, SubagentStop, Stop, Interrupt`.
Common stdin: `session_id`, `transcript_path`, `cwd`, `hook_event_name`, `model`, often `permission_mode`. Per event: `SessionStart.source` (`startup|resume|clear|compact|fork`); `SessionEnd.reason` (currently always `other`);
`PreToolUse` `turn_id,tool_name,tool_use_id,tool_input`; `PermissionRequest` `turn_id,tool_name,tool_input`; `PostToolUse` adds `tool_response`; `UserPromptSubmit` `turn_id,prompt`; `Stop`
`turn_id,stop_hook_active,last_assistant_message`; `Interrupt` `turn_id,permission_mode`; subagent events carry `agent_id,agent_type`. Matchers are regexes honoured by tool, compact, session and subagent events and ignored by
`UserPromptSubmit`, `Stop`, `Interrupt`; `timeout` is in seconds (default 600; `SessionEnd`/`Interrupt` default 1, max 3) [DOC hooks]. Codex has an explicit `Interrupt` event, unlike Claude's `Stop`.
Orca treats a Plan-mode "Implement this plan?" menu as blocked even after `Stop` fired [ORCA agent-state-rules/codex.json]. Fixtures: `tests/fixtures/codex/*.json`.

**Resume.** `codex resume [SESSION_ID]`; `codex resume --last` "resume the most recent chat from the current working directory"; `--all` widens to other directories; `codex exec resume ...` is the headless form
[DOC developer-commands]. Because `--last` is cwd-scoped, each worktree resumes its own last chat, which is what we want; store `session_id` from `SessionStart` for exact resume.

**Initial prompt.** Positional `[PROMPT]`: "Optional text instruction to start the session. Omit to launch the TUI without a pre-filled message" [DOC developer-commands]. `codex exec` is non-interactive. Orca delivers it via argv for Codex
[ORCA tui-agent-config.ts] and notes a provisional `model: loading` header discards input typed behind it, so a paste must wait for a footer row [ORCA agent-state-rules/codex.json].

**Approval.** `--ask-for-approval`/`-a` lists `on-request | never`; `approval_policy` accepts `on-request`, `never` or a `granular` table, `untrusted` unsupported and `on-failure` deprecated; `--sandbox`/`-s`
`read-only | workspace-write | danger-full-access`; `--dangerously-bypass-approvals-and-sandbox`, alias `--yolo`; `--approve-for-me`/`--not-so-yolo` (automatic approval review, `on-request`, `workspace-write`; conflicts with `--sandbox` and `--yolo`)
[DOC developer-commands, config-reference]. `--full-auto` is not on the page read, so the profile must not use it. Profile: `yolo_args = ["--yolo"]`, never default.

**Title, OSC, bell.** `tui.terminal_title` is an array of item ids, default `["spinner","project"]`, `null` disables; `tui.notification_method` is `auto|osc9|bel` (default `auto`) [DOC config-reference]. Glyph set not documented
(Orca has no Codex title rule; it relies on screen text and hooks [ORCA agent-state-rules/codex.json]). Orca's Codex ready anchor is the header `OpenAI Codex (v...)` with `model:` and `directory:` labels [ORCA codex.json].

**Trust dialog.** `projects.<path>.trust_level = "trusted" | "untrusted"`; "Untrusted projects skip project-scoped `.codex/` layers, including project-local config, hooks, and rules" [DOC config-reference]. The first-run screen
contains "Do you trust the contents of this directory" style text; Orca matches `do you trust`/`trust this`/`trusted workspace` plus `workspace|folder|directory|repo` and a `trust and continue` choice
[ORCA src/main/runtime/agent-state-rules/blocked-text-layer.ts, trust-dialog-status-observer.ts]; exact wording is [INFER]. A new worktree is a new path, so expect this dialog per worktree until the user trusts the parent.

## 3. Gemini CLI (`gemini`)

Notice on the CLI reference page: Gemini CLI "was replaced by Antigravity CLI on June 18th, 2026" for unpaid-tier and Google One users [DOC https://geminicli.com/docs/cli/cli-reference/]. Orca carries a separate `antigravity` (`agy`)
agent with `-i`-style prompt flag and exact-path trust in `~/.gemini/antigravity-cli/settings.json` [ORCA tui-agent-config.ts, src/main/agent-trust-presets.ts]. Plan a separate `agy` profile after verification; this section covers `gemini`.

**Per-launch hooks.** Hooks live in `settings.json` under `hooks`; schema `hooks.<Event>[] = { matcher, sequential, hooks: [{ type:"command", command, name, timeout(ms, default 60000), description }] }`, only `type:"command"` [DOC https://geminicli.com/docs/hooks/reference/].
Settings precedence: defaults < system defaults < user `~/.gemini/settings.json` < project `.gemini/settings.json` < **system settings file** < env < CLI args; the system file path is overridable with `GEMINI_CLI_SYSTEM_SETTINGS_PATH`
(defaults file: `GEMINI_CLI_SYSTEM_DEFAULTS_PATH`); `GEMINI_CLI_HOME` relocates the user-level root [DOC https://geminicli.com/docs/reference/configuration/].
Recipe **[INFER]**: launch with `GEMINI_CLI_SYSTEM_SETTINGS_PATH=<session file>` containing only the `hooks` block. The page does not say whether hooks from different layers merge or the highest layer wins, so verify that the user's own hooks survive;
if they do not, the fallback is no hooks (channels 3 to 4 only) rather than editing `~/.gemini/settings.json`, which Orca does [ORCA src/main/gemini/hook-service.ts, `~/.gemini/settings.json`]. `GEMINI_CLI_HOME` as an overlay would relocate auth and history; avoid.

**Events and fields.** `BeforeTool, AfterTool, BeforeAgent, AfterAgent, BeforeModel, BeforeToolSelection, AfterModel, SessionStart, SessionEnd, Notification, PreCompress`. Common stdin: `session_id`, `transcript_path`, `cwd`, `hook_event_name`, `timestamp`.
Per event: `BeforeTool` `tool_name,tool_input,mcp_context,original_request_name`; `AfterTool` adds `tool_response`; `BeforeAgent` `prompt`; `AfterAgent` `prompt,prompt_response,stop_hook_active`; `SessionStart.source` (`startup|resume|clear`);
`SessionEnd.reason` (`exit|clear|logout|prompt_input_exit|other`); `Notification` `notification_type` (`"ToolPermission"`), `message`, `details`; `PreCompress.trigger`. Exit 0 parses stdout JSON (the script "must not print any plain text to stdout other than the final JSON"), exit 2 blocks,
other codes warn [DOC hooks/reference]. Orca registers only `BeforeAgent, AfterAgent, AfterTool, BeforeTool` and says "Gemini has no permission-prompt hook" [ORCA gemini/hook-service.ts]; the current docs list a `Notification` event with `ToolPermission`, so permission state can come from `Notification` once verified
(Orca's statement may predate it). Mapping: `BeforeAgent` -> `Working`; `AfterAgent` -> `TurnFinished`; `Notification/ToolPermission` -> `NeedsInput(Permission)`. Fixtures: `tests/fixtures/gemini/*.json`.

**Resume.** `--resume` / `-r` takes `latest` or an index number (`--resume 5`) [DOC cli-reference]; no id form is documented.

**Initial prompt.** `-i` / `--prompt-interactive` "Execute prompt and continue in interactive mode"; `-p` / `--prompt` "Forces non-interactive mode"; a positional `query` "Defaults to interactive mode in a TTY" [DOC cli-reference]. Orca uses `-i` [ORCA tui-agent-config.ts `flag-prompt-interactive`].
Use `gemini -i "<prompt>"`.

**Approval.** `--approval-mode` `default | auto_edit | yolo | plan`; `--yolo`/`-y` is deprecated in favour of `--approval-mode=yolo`; `--sandbox`/`-s` boolean [DOC cli-reference]. Profile: `yolo_args = ["--approval-mode", "yolo"]`.

**Title, OSC.** Title glyphs: `✦` working, `⏲` silent working, `◇` idle, `✋` permission [ORCA src/shared/agent-title-glyphs.ts, agent-state-rules/gemini.json]. This is the only agent whose permission state is in the title, which makes it the best heuristic-only candidate. OSC 7501 not documented for Gemini.

**Trust dialog.** Folder trust on first run in a folder: "Trust folder", "Trust parent folder", "Don't trust" (safe mode); decisions in `~/.gemini/trustedFolders.json` (override path `GEMINI_CLI_TRUSTED_FOLDERS_PATH`); `--skip-trust` or `GEMINI_CLI_TRUST_WORKSPACE=true` trusts for one session; headless runs in an untrusted folder
fail with `FatalUntrustedWorkspaceError`; untrusted folders skip project `.gemini/settings.json` [DOC https://geminicli.com/docs/cli/trusted-folders/, reference/configuration/]. We do not pre-trust (decision 12.5); the menu is a `NeedsInput(Trust)` screen.

## 4. OpenCode (`opencode`)

**Per-launch hooks.** There are no stdin command hooks; extension is by plugins. Plugins load from `.opencode/plugins/`, `~/.config/opencode/plugins/`, or the `plugin` array of `opencode.json` (npm packages); "Plugins are loaded from all sources and all hooks run in sequence"; a plugin is
`export const X = async ({ project, client, $, directory, worktree }) => ({ ...hooks })` [DOC https://opencode.ai/docs/plugins/]. Config files "are merged together, not replaced"; `OPENCODE_CONFIG_DIR` is "a custom directory searched for agents, commands, modes, and plugins ...
loaded after the global config and `.opencode` directories"; also `OPENCODE_CONFIG` (extra config file), `OPENCODE_CONFIG_CONTENT` (inline JSON config), `OPENCODE_PERMISSION` (inline JSON permissions), `OPENCODE_TUI_CONFIG` [DOC https://opencode.ai/docs/config/, https://opencode.ai/docs/cli/].
Recipe: create `<session dir>/opencode-config/plugins/archipel.js` and launch with `OPENCODE_CONFIG_DIR=<session dir>/opencode-config`. The variable holds one directory, so if the user already exports `OPENCODE_CONFIG_DIR`, mirror its entries into the overlay as Orca does
[ORCA src/main/opencode/hook-service.ts, overlay-mirror, "existingConfigDir" branch]; this keeps global config untouched because the overlay lives in our data dir. Events reach the plugin through its `event` handler as `{type, properties}` [ORCA src/main/opencode2/status-plugin-setup-source.ts].
Event names [DOC plugins]: `session.created, session.compacted, session.deleted, session.diff, session.error, session.idle, session.status, session.updated, permission.asked, permission.replied, message.part.updated, message.updated, tool.execute.before, tool.execute.after, file.edited, todo.updated, tui.toast.show, ...`.
Property shapes are not documented on the pages read; from Orca's captures: `session.status.properties.status.type` is `busy|idle`, `permission.asked.properties` has `id, sessionID, permission, patterns, metadata`, replies carry `requestID` [ORCA src/shared/opencode-permission-status.test.ts, opencode2/status-plugin-setup-source.ts]. [INFER]
The plugin forwards to `archipel-cli agent hook --source opencode` (spawn from the plugin via `$`). Mapping: `session.status busy` -> `Working`; `session.idle` or `status idle` -> `TurnFinished`; `permission.asked` -> `NeedsInput(Permission)`; `session.error` -> `Errored`. Fixtures: `tests/fixtures/opencode/*.json` (lowest confidence).

**Resume.** TUI flags `--continue`/`-c` ("Continue the last session"), `--session`/`-s <id>`, `--fork` (with either) [DOC https://opencode.ai/docs/cli/]. The session id comes from `sessionID` in plugin events.

**Initial prompt.** TUI flag `--prompt "text"` (plus `--model provider/model`, `--agent`) [DOC cli]. `opencode run "msg"` is the non-interactive mode. Orca uses the flag form [ORCA tui-agent-config.ts `flag-prompt`] and waits for bracketed paste enable plus a show-cursor before pasting because opencode enables `CSI ?2004h`
before its composer mounts; on slow hosts the composer took up to about 10 s [ORCA tui-agent-config.ts comments]; with `--prompt` argv we avoid that race.

**Approval.** `permission` config per tool: `"allow"`, `"ask"`, `"deny"`, nested patterns for `bash`; most default to allow, `doom_loop` and `external_directory` default to ask, reading `.env` asks; the TUI prompt offers `once`, `always`, `reject`; flag `--auto` "approving requests that are not explicitly denied"
[DOC https://opencode.ai/docs/permissions/, config]. The CLI page lists `--auto` under `opencode run`; whether the bare TUI accepts it is [INFER] (the permissions page says it can also be toggled from the command palette). Per-launch alternative: `OPENCODE_PERMISSION` inline JSON.
Because OpenCode allows most things by default, "needs input" is rarer than for Claude.

**Title.** The native title is `OC | <session title>`, optionally preceded by a wrapper label and one status glyph (braille spinner or `U+25A3`) [ORCA src/shared/opencode-terminal-title.ts, agent-state-rules/opencode.json].

**Trust dialog.** None found in the OpenCode docs read; provider sign-in on first launch is the likely first-run screen [INFER].

## 5. Aider (`aider`)

**Hooks.** None. Per-launch signals only: `--notifications` ("Enable/disable terminal bell notifications when LLM responses are ready", default False) and `--notifications-command COMMAND` ("a command to run for notifications instead of the terminal bell"); fires "when the LLM has finished generating a response and is waiting
for your input" [DOC https://aider.chat/docs/config/options.html, https://aider.chat/docs/usage/notifications.html]. Every option has an `AIDER_*` env var (`AIDER_NOTIFICATIONS`, `AIDER_NOTIFICATIONS_COMMAND`), so a launch can set them without touching `~/.aider.conf.yml`. Using plain `--notifications` yields a BEL, which vt100's
`Callbacks::audible_bell` already surfaces: a free, reliable `TurnFinished` edge. `--notifications-command "archipel-cli agent hook --source aider"` is the alternative; no stdin or argv payload is documented, so it carries only the session token from env.

**Resume.** There is no session id. `--restore-chat-history` ("Restore the previous chat history messages", default False) with `--chat-history-file` (default `.aider.chat.history.md`) and `--input-history-file` (default `.aider.input.history`) [DOC options]. In a worktree these files sit in the worktree, so
resume is per worktree. Command: `aider --restore-chat-history`.

**Initial prompt.** `--message`/`-m` and `--message-file`/`-f` "send the LLM, process reply then exit (disables chat mode)" [DOC options], so they cannot start an interactive session. `--load FILE` "Load and execute /commands from a file on launch". Use stdin after ready (Orca: `stdin-after-start`
[ORCA tui-agent-config.ts]). Slash commands `/ask`, `/code`, `/architect`, `/run`, `/exit` exist [DOC https://aider.chat/docs/usage/commands.html].

**Approval.** `--yes-always` "Always say yes to every confirmation" [DOC options]; without it Aider asks inline `(Y)es/(N)o` style confirmations (screen rule `(y/n)`; exact text [INFER], not in the docs read). `--no-auto-commits` / `--auto-commits` (default True) matters for review because Aider commits by itself [DOC options].

**Title, OSC, trust.** Aider is not documented to set a terminal title or emit OSC; Orca has no Aider-specific title or screen rules, only name-token detection [ORCA src/shared/agent-title-identity.ts]. No trust dialog is documented. A "Add .aider* to .gitignore" prompt and model-warning prompts at start-up are
known from memory, not from the docs read: treat as [INFER], covered by the generic `(y/n)` blocked rule.

## 6. Cross-cutting facts

- **Terminal title coverage.** Claude: glyph at rest, spinner working (or OSC 7501). Gemini: four glyphs. OpenCode: `OC | ...` name only. Codex: configurable spinner item. Aider: none documented. Title alone is therefore a real signal only for Claude and Gemini.
- **OSC 7501.** Claude (v2.1.295+, only if the terminal answers the probe, and not in tmux). Parser needs `Callbacks::unhandled_osc` (see design 13). Query reply bytes for our terminal: `ESC ] 7501 ; ? : states=idle,working,done,blocked,error : kinds=permission,question,auth ESC \` (without the spaces).
- **BEL.** Aider (`--notifications`), Codex (`tui.notification_method=bel|osc9`), Claude (`preferredNotifChannel: "terminal_bell"`). Surface via `Callbacks::audible_bell`.
- **Bracketed paste.** Claude, OpenCode and Codex enable it; Orca waits for `CSI ?2004h` before pasting into OpenCode. Our `TerminalSession::paste` is already bracketed-paste aware.
- **Per-launch isolation summary.** Claude: `--settings` (clean). Codex: `-c` + bypass-trust flag (clean if it works), else `notify`. Gemini: system-settings env var (clean if hooks merge). OpenCode: `OPENCODE_CONFIG_DIR` overlay (clean). Aider: env vars (clean). No agent requires editing a global config file.
- **Decisions 12.1 to 12.5 still hold.** Claude and Codex are the hook-backed first-class agents; the rest run on title/screen/BEL.

## Verification status (what a real install must still prove)

No install was done, so none of the following is confirmed by execution. Highest risk first.

1. Codex: that `-c 'hooks.<Event>=[...]'` is accepted, that `features.hooks=true` is needed, and that `--dangerously-bypass-hook-trust` makes inline hooks run without writing `[hooks.state]`. If any fails, use `notify` plus screen rules and file an upstream request.
2. Claude: that `hooks` in `--settings` run in addition to the user's hooks, and that they run while the trust dialog is still open (the "what runs before you trust a folder" table was truncated in the fetch).
3. Gemini: that `GEMINI_CLI_SYSTEM_SETTINGS_PATH` hooks coexist with user hooks and that `Notification/ToolPermission` fires on every approval prompt.
4. OpenCode: `properties` shapes for `session.status`, `permission.asked`, `session.error` on the current version, and whether `--auto` works for the TUI.
5. Real PTY captures (title bytes, bracketed paste, trust dialog frames) for all five; replace the synthetic fixtures where a capture exists.
6. Antigravity CLI (`agy`) as the successor of the unpaid `gemini` tier: separate profile, separate research.

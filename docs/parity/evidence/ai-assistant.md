# AI Assistant: providers and AI Chat

Issue #52, slice 1. Clean-room equivalent of the observable AI Chat of IntelliJ AI
Assistant / Android Studio Gemini (right-stripe sparkle tool window, scene `04` right
stripe "Agent/Gemini" icon in `reference/android-studio/NOTES.md`). No reference
screenshot of the chat window itself exists yet, so layout parity is **not** claimed;
the behaviour below is what is implemented and tested.

```sh
cargo test -p archipel-ide-ai          # SSE parsing, providers, service, chat, markdown, diff
cargo test -p archipel-ide-core ai_    # commands: configure, chat, stop, errors, apply, policy
ARCHIPEL_AI_MOCK=1 cargo run -p archipel-ide-app -- <project>   # offline demo provider
```

## Behaviour

- **Off until configured.** `AiSettings::enabled` defaults to `false`; a key in the
  environment never enables AI. `ai.configure` validates provider, model and key
  *before* persisting, so a missing key leaves AI off (`01-ai-off-setup.png`).
- **Providers** (`crates/archipel-ide-ai`): Anthropic Messages API over SSE (default
  `claude-opus-5-5` for chat, `claude-haiku-5-5` for inline completion, both
  configurable; automatic prompt caching; `output_config.effort`; client tools with
  `eager_input_streaming` and strict JSON validation of streamed inputs; refusals
  surfaced as errors with partial output discarded; server-side refusal fallbacks
  (`fallbacks: "default"`) on supported models against the first-party endpoint,
  with fallback-aware history echo), OpenAI-compatible Chat Completions (SSE), and
  Ollama `/api/chat` (NDJSON). All share one `Provider` trait and are tested with
  canned streams; no test performs network I/O.
- **Keys**: stored key in `<config>/secrets.toml` (mode `0600`) wins over
  `ANTHROPIC_API_KEY` / `OPENAI_API_KEY`. `<config>` is `ARCHIPEL_CONFIG_DIR` or the
  platform config directory. OS keychain storage is not implemented yet.
- **Project data sharing**: `.archipel/ai.toml` (`allow_ai`, `share_file_contents`);
  an unreadable policy fails closed.
- **AI Chat tool window** (right stripe, `toolwindow.activate.aiChat`): streaming
  Markdown (headings, lists, inline code, fenced code with editor syntax colors),
  "Thinking…/Generating…" with Stop (`02-chat-streaming.png`), context chips for the
  current file or selection, slash commands `/explain /tests /docs /fix /refactor`
  with suggestions, per-project history in `.archipel/ai-chat.json`
  (`03-chat-answer.png`).
- **Code blocks**: Copy, Insert at caret, and Apply… which previews a line diff
  against the attached selection/file (or current selection) and applies it as one
  undoable edit; a preview made stale by later edits is refused
  (`04-apply-diff-preview.png`, `05-applied-in-editor.png`).
- **Status bar widget**: provider/model, "generating…", "off" or "disabled";
  click opens AI Chat.

## Inline completion (slice 2)

```sh
cargo test -p archipel-ide-ai completion
cargo test -p archipel-ide-core ai_inline
```

- After a 350 ms typing pause (edits only, not caret moves or file switches) a request
  goes to the completion model with up to 6000/2000 characters before/after the caret
  and a `<cursor/>` marker; a newer edit cancels it and late answers for an outdated
  revision or caret are dropped.
- Offered only at the end of a line, optionally before up to three closing
  delimiters; never while the LSP completion popup is open (the popup keeps Tab/Enter/Esc).
- Output cleanup strips fences, echoed current-line text, closers that already follow
  the caret and trailing lines duplicating the following code; block bodies completed
  inside auto-closed brackets end on a new line so the closer keeps its own line.
- Ghost text: muted first line at the caret, following lines drawn as an overlay
  block, the displaced rest of the caret line redrawn after the suggestion, and a
  key hint (`06-…`, `08-…`). Typing matching characters consumes the suggestion,
  anything else dismisses it. Tab accepts (one undo step, `07-…`), Ctrl+→ (⌥→ on
  macOS) accepts the next word, Esc dismisses. Accepting does not trigger a new request.
- Limitation: following lines overlay the code below instead of pushing it down
  (the editor has no block inlays yet).

## AI actions and Fix with AI (slice 3)

```sh
cargo test -p archipel-ide-ai actions
cargo test -p archipel-ide-core ai_actions
```

- Editor context menu → **AI Actions**: Explain Code, Suggest Refactoring, Find
  Problems, Generate Unit Tests, Generate Documentation, and Fix with AI when the file
  has problems (`10-…`). With AI off the submenu offers "Set Up AI Assistant…".
- Actions run in AI Chat with the selection (or the whole file) attached. Refactor,
  documentation and fix answers open the Apply diff preview as soon as they complete
  (`11-…`); Explain/Find Problems/Tests only answer.
- **Fix with AI** on problems: syntax errors and language-server diagnostics of the
  caret line (or of the whole file) are listed in the prompt; also a header button of
  the Problems tool window.
- **Fix Build with AI**: a "Fix with AI" link next to a failed build/run status, as in
  the Android Studio Build/Sync output (reference scene a06). The last 200 output lines
  are attached as a "Build output" chip; the first error location (rustc, javac,
  kotlinc, gcc/clang formats) is resolved to a project file, opened and attached so the
  fix can be applied (`09-…`).

## Commit message generation (slice 4)

```sh
cargo test -p archipel-ide-git diffs_selected_paths
cargo test -p archipel-ide-ai commit
cargo test -p archipel-ide-core ai_vcs
```

- The Commit window's action row has an AI button (reference scene 12 shows an
  AI-generate icon next to the message field). It sends the diff of exactly the files
  selected for commit — working tree against `HEAD`, untracked files as additions,
  binary files summarized, staged content when there is no commit yet — plus the branch
  and the last 10 subjects so the message follows the repository's convention.
- The message streams into the field (Stop cancels); fences, quotes and labels are
  stripped. Diffs over 60 000 characters are cut and the prompt says so.
- Uses the fast completion model. `12-commit-message-generated.png`.

## Terminal: natural language → command (slice 6)

```sh
cargo test -p archipel-ide-ai shell
cargo test -p archipel-ide-core ai_terminal
```

- The Terminal header has an AI toggle that opens a "Describe a command" bar. Enter asks
  the fast model for exactly one single-line command for the detected OS and shell
  (`SHELL`/`COMSPEC`), the working directory and, when file sharing is allowed, the last
  30 visible terminal lines. Replies that are empty, multi-line or `UNSURE: …` are shown
  as errors instead of commands.
- Nothing runs without confirmation: **Run** (or ⌘⏎) types the command and Enter,
  **Insert** types it without Enter, Cancel discards it. Commands that delete files,
  use `sudo`/`su`, force-push/reset/clean, pipe downloads into a shell or write to
  disks are flagged with a warning (`13-…`, `14-…`).
- The bar has a constant height while open: an earlier version grew when a suggestion
  appeared, which shrank the PTY to one or two rows so the output of a confirmed command
  scrolled into scrollback and seemed lost. Found during the Xvfb walkthrough by tracing
  the PTY bytes; the real-PTY test waits for the shell prompt like the IDE does.

## Agent mode (slice 5)

```sh
cargo test -p archipel-ide-ai agent
cargo test -p archipel-ide-core ai_agent demo_agent
```

- **Agent** tab in AI Chat: describe a task; the agent replies with a numbered plan and
  works through it with tools: `list_files`, `read_file` (with line numbers), `search`,
  `edit_file` (old text must match exactly once), `create_file` and `run_command`.
- Reading, listing and searching run automatically. **Every edit, new file and command
  waits for Approve / Reject**, with a diff or the command line (plus the terminal risk
  warning) on the approval card (`15-…`). Rejections and failures (text not found,
  ambiguous match, file exists, another task running) go back to the model as error
  results so it can adapt.
- Edits are applied to the editor document as one undoable step, then saved so builds and
  tests see them (`16-…`); commands run in the Build tool window and return the exit code
  and the last 200 output lines.
- Loop rules from the Claude API guidance: tool schemas are strict, streamed tool inputs
  are validated before running and answered with `INVALID_JSON` errors when malformed,
  `max_tokens` with a pending tool call stops instead of running a truncated call, all
  results of a turn go back in one message, thinking blocks are kept in the history, and
  a task stops after 30 model turns. **Stop** cancels the request and a running command.

## Evidence

Linux, Xvfb 1600×1000, `ARCHIPEL_RENDERER=glow`, 2026-10-10. Answers come from the
offline demo provider (`ARCHIPEL_AI_MOCK=1`, labelled "mock-chat (demo)"); no
`ANTHROPIC_API_KEY` was available, so no real-provider walkthrough was recorded.

| Screenshot | Shows |
|---|---|
| `ai/01-ai-off-setup.png` | AI off: setup form, actionable missing-key error |
| `ai/02-chat-streaming.png` | streaming answer with Stop, selection chip `main.rs:3-9` |
| `ai/03-chat-answer.png` | completed Markdown answer, highlighted code block actions |
| `ai/04-apply-diff-preview.png` | Apply… diff preview (+1 −0) |
| `ai/05-applied-in-editor.png` | applied edit in the editor |
| `ai/06-inline-ghost-multiline.png` | multi-line ghost text after `fn helper() {`, closer redrawn below |
| `ai/07-inline-accepted.png` | Tab accepted: inserted body, closer on its own line, no new request |
| `ai/08-inline-ghost-inside-brackets.png` | single-line ghost inside `vec![…]`, `]` redrawn after it |
| `ai/09-fix-build-with-ai.png` | failed `cargo build` with "Fix with AI" link; chat with Build output + main.rs chips |
| `ai/10-editor-ai-actions-menu.png` | editor context menu → AI Actions submenu |
| `ai/11-refactor-auto-diff.png` | Suggest Refactoring answer opened the diff preview automatically |
| `ai/12-commit-message-generated.png` | Commit window with a generated message and the AI button |
| `ai/13-terminal-suggestion.png` | Terminal AI bar with a suggested command awaiting Run / Insert / Cancel |
| `ai/14-terminal-confirmed-run.png` | output after Run |
| `ai/15-agent-awaiting-approval.png` | agent plan, automatic list/read steps, edit awaiting approval with diff |
| `ai/16-agent-task-complete.png` | approved edit applied and saved, summary, task complete |

## Gaps

Alt+Enter intention integration, fixing failed tests, Explain Commit, agent checkpoints/multi-file review, `@file`/`@symbol`,
open-tabs/VCS context and OS keychain storage are follow-up slices.

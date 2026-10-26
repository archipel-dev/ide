# Lazy viewport-first highlighting (#41)

Opening a large file used to highlight the whole buffer up front, and injected
languages (every Rust macro call, Markdown fences, `<script>`) were overlaid one by
one onto the full layer, which is quadratic in the number of injections.

Changes (`archipel-ide-language`, `archipel-ide-core`, `archipel-ide-ui`):

- Injected layers are collected and merged in one pass; injected parsers are reused per
  language and thread.
- Editor buffers use `analyze_lazy`: only the first `INITIAL_HIGHLIGHT_BYTES` (32 KiB)
  are highlighted on open. The editor calls `LanguageAnalysis::ensure_highlighted` for last
  frame's visible rows plus 60 rows of margin before painting. Edits only re-highlight
  ranges that were already computed; the rest stays lazy.
- `analyze` still highlights everything (diff views, snippets, tests).
- Property test: lazy incremental analyses, completed with `ensure_highlighted`, equal
  a full analysis after random edit sequences (injected regions with syntax errors are
  exempt, because Tree-sitter error recovery there depends on position).

## Measurement

`cargo run --release -p archipel-ide-language --example highlight_bench` — synthetic Rust
file, 963,560 bytes, 48,000 lines, one `vec!`/`format!` pair per 6 lines (16,000 macro
injections). Single run on the CI container, same machine for both columns; "before" is
`origin/main` at the time of the change.

| Step | Before | After |
|---|---|---|
| Open (parse + highlight) | 38,065 ms | 282 ms (viewport-first), 1,918 ms (full `analyze`) |
| Scroll to middle (4 KB newly highlighted) | — (already computed) | 6.4 ms |
| 20 keystrokes mid-file | 1,554 ms | 1,466 ms (unchanged: dominated by the incremental parse) |

Remaining cost on open is the Tree-sitter parse itself. Keystroke latency on huge files
is not improved by this change.

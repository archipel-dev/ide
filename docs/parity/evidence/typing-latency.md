# Typing latency on a 50,000-line file

Target (editor P0 workstream): a keystroke frame takes < 16 ms on a 50k-line file, matching the
reference IDE's rule that typing is never blocked by background analysis (`PLATFORM-PERF-008`).

Reproduce: `cargo bench -p archipel-ide-ui --bench typing` (release). The bench opens a generated
50,000-line Java class as a project file, drives `IdeApp::frame` on a headless `egui::Context`
(input handling, editing, analysis refresh, LSP sync attempt, layout of visible lines; no GPU
painting) and types 200 characters below the first screen. It asserts that the keystrokes reached
the buffer and exits non-zero when p95 exceeds 16 ms.

| Date | Build | File | p50 | p95 | max | Notes |
|---|---|---|---:|---:|---:|---|
| 2026-10-10 | before (main + #96) | `Big.txt` | 1.35 ms | 1.86 ms | 3.57 ms | status bar copied and scanned the whole buffer every frame |
| 2026-10-10 | this change | `Big.txt` | 0.70 ms | 1.70 ms | 3.83 ms | bounded indent/line-separator detection; cached find matches |
| 2026-10-10 | this change | `Big.java` | 358.8 ms | 405.5 ms | 428.5 ms | whole-file tree-sitter analysis per keystroke (~430 ms standalone) |
| 2026-10-10 | integration 4A, merged (#72 incremental reparse) | `Big.java` | 382.2 ms | 429.1 ms | 725.1 ms | analysis is incremental now, but #120's run icons reparse the whole file and #113's breadcrumbs rerun the tags query on every keystroke |
| 2026-10-10 | 4A + breadcrumbs reuse structure while typing | `Big.java` | 277.5 ms | 329.0 ms | 585.2 ms | run icons still reparse |
| 2026-10-10 | 4A + run icons from the incremental tree | `Big.java` | 13.1 ms | 29.3 ms | 372.8 ms | the run-icon walk over 12,500 methods remains per keystroke |
| 2026-10-10 | 4A + run icons refreshed once typing pauses (run 1) | `Big.java` | 2.11 ms | 13.88 ms | 386.5 ms | `Big.txt` p95 1.94 ms |
| 2026-10-10 | same (run 2) | `Big.java` | 2.16 ms | 18.55 ms | 426.7 ms | load average 2.8 on 4 cores (another build running) |
| 2026-10-10 | same (run 3) | `Big.java` | 2.07 ms | 13.66 ms | 381.5 ms | `Big.txt` p95 2.65 ms |

Machine: shared Linux CI container; absolute numbers vary between runs, the ratio between the
two files does not. With #72 merged, the Java gap came from whole-file work that other features
added on top of the incremental analysis: run icons (#120) and breadcrumbs/Structure (#113).
Run icons now use the editor's incremental tree, and both are recomputed 250 ms after the last
edit instead of on every keystroke (`structure_view::items`, `editor::run_icons`). p95 is at or
just under the budget on this shared container (2 of 3 runs pass). The ~400 ms max is one slow
frame per run. Its cause has not been isolated yet; the first measured frame also pages down.

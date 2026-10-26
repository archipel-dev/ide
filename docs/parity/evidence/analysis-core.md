# Analysis pipeline core (WP1) evidence

Rows: PLAT-073 (new), PLAT-011, PLAT-012, PERF-008. Design: `docs/analysis/architecture.md`.
Measured in the shared Linux container (4 cores, other builds running), release profile.

## Tests

- `cargo test -p archipel-ide-text --lib edit_map`: `normalized_map_equals_direct_mapping`,
  `composition_equals_direct_mapping` (512 cases each; also run with `PROPTEST_CASES=3000`).
- `cargo test -p archipel-ide-language`: `anchors_never_resolve_to_a_different_kind_after_random_edits`,
  `anchors_before_an_edit_resolve_exactly` (anchors.rs); `changed_ranges_cover_every_structural_change`
  (incremental.rs, also run with `PROPTEST_CASES=1500`); `snapshot_api_matches_the_facade_and_the_feature_modules`
  and `every_language_builds_tables_and_snapshots` (snapshot_golden.rs: highlight, structure, folds,
  selection, brackets and diagnostics identical through `LanguageAnalysis`, `SyntaxSnapshot` and the
  feature modules for 11 samples); registry order/base-language tests (`extensions.rs`); token-set
  names checked against every grammar (`registry.rs`).
- `cargo test -p archipel-ide-analysis`: `revoked_jobs_return_cancelled_within_256_checks`,
  `coalesced_jobs_cancel_their_predecessor`, `cached_values_follow_file_and_interface_stamps_with_early_cutoff`
  (an edit inside a function body keeps interface-dependent values, a rename invalidates them),
  `snapshots_are_isolated_from_later_changes`, `edits_publish_changed_ranges_and_journal_maps_offsets`,
  `vfs_batches_update_stamps_and_ids_survive_moves`, `tests/contracts.rs` (every extension point
  through the registry with its mock implementation).
- `cargo test -p archipel-ide-core analysis`: `documents_are_mirrored_into_the_analysis_host`
  (open, edit, close, events, `analysis.status`).

## Analysis bench

`cargo bench -p archipel-ide-analysis --bench analysis`:

```
Java source: 50000 lines, 1321 KiB
typing in a 2k-line file, middle:
  raw tree-sitter reparse + changed_ranges   p50     0.997 ms   p95     2.791 ms   (300 samples)
  reparse + changed ranges (keystroke)       p50     1.174 ms   p95     2.808 ms   (300 samples)
typing in a 50k-line file, start:
  raw tree-sitter reparse + changed_ranges   p50    15.904 ms   p95    23.839 ms   (300 samples)
  reparse + changed ranges (keystroke)       p50    24.443 ms   p95    34.165 ms   (300 samples)
typing in a 50k-line file, middle:
  raw tree-sitter reparse + changed_ranges   p50    15.012 ms   p95    22.036 ms   (300 samples)
  reparse + changed ranges (keystroke)       p50    23.305 ms   p95    35.448 ms   (300 samples)
host apply(Edit) + snapshot                  p50     0.012 ms   p95     0.018 ms   (300 samples)
edits_since over 300 revisions               p50     0.015 ms   p95     0.020 ms   (50 samples)
EditMap push (10k scattered keystrokes)      p50   599.030 ms   p95   599.030 ms   (1 samples)
EditMap map (10k pieces)                     p50     0.000 ms   p95     0.000 ms   (10000 samples)
Anchor::resolve (exact, 50k-line file)       p50     0.004 ms   p95     0.008 ms   (129 samples)
Db::cached hit (1k entries)                  p50     0.000 ms   p95     0.000 ms   (10000 samples)
memory: 1000 snapshots of the 50k-line file: +0 KiB total (0 B each)
memory: one parsed 50k-line tree: ~44724 KiB
cache: CacheStats { entries: 1000, bytes: 104000, hits: 10000, misses: 1000 }
```

The 2k-line reparse is within the 2 ms budget at p50; its p95 follows the raw tree-sitter floor on
this loaded machine. On the 50k-line single-class file raw tree-sitter alone needs ~15 ms per
keystroke; the remaining cost of `SyntaxTree::reparse` is the pre-existing whole-line and error-region
bookkeeping (unchanged by WP1 apart from MISSING nodes counting as changed). Host bookkeeping per
edit is ~12 µs and snapshots share trees and text (1000 snapshots: no measurable RSS growth).
Composing 10k scattered single-byte edits into one map costs ~0.6 s (quadratic); typed words stay
one piece and the host composes per-revision maps on demand only.

## Typing latency before/after (PERF-008)

`cargo bench -p archipel-ide-ui --bench typing`, baseline binary built from
`chore/rename-archipel` (59e711a) and the WP1 binary, run interleaved 10 times each:

| File | Metric | Baseline mean / median | WP1 mean / median |
|---|---|---|---|
| Big.java | p50 | 2.02 / 2.01 ms | 2.06 / 2.06 ms |
| Big.java | p95 | 16.02 / 15.41 ms | 16.31 / 15.87 ms |
| Big.txt | p50 | 2.03 / 1.98 ms | 1.83 / 1.74 ms |
| Big.txt | p95 | 3.29 / 3.25 ms | 3.57 / 3.30 ms |

Medians are within 3 % (Java p50 +2.2 %, p95 +3.0 %); the plain-text p95 mean differs by 8.5 %
with a 1.5 % median difference (single noisy runs; see raw runs below). The editor's incremental
tree is handed to the host as is, and plain-text buffers are copied only after a typing pause.

```
== base 1
Big.txt: 50000 lines, 200 keystrokes: p50 1.96 ms, p95 3.28 ms, max 7.90 ms (budget 16 ms)
Big.java: 50000 lines, 200 keystrokes: p50 1.97 ms, p95 14.69 ms, max 412.12 ms (budget 16 ms)
== new 1
Big.txt: 50000 lines, 200 keystrokes: p50 2.21 ms, p95 5.20 ms, max 7.15 ms (budget 16 ms)
Big.java: 50000 lines, 200 keystrokes: p50 2.19 ms, p95 19.41 ms, max 458.64 ms (budget 16 ms)
== base 2
Big.txt: 50000 lines, 200 keystrokes: p50 1.85 ms, p95 2.63 ms, max 3.86 ms (budget 16 ms)
Big.java: 50000 lines, 200 keystrokes: p50 1.97 ms, p95 14.73 ms, max 398.23 ms (budget 16 ms)
== new 2
Big.txt: 50000 lines, 200 keystrokes: p50 1.74 ms, p95 4.20 ms, max 6.39 ms (budget 16 ms)
Big.java: 50000 lines, 200 keystrokes: p50 2.00 ms, p95 15.74 ms, max 547.07 ms (budget 16 ms)
== base 3
Big.txt: 50000 lines, 200 keystrokes: p50 2.29 ms, p95 3.66 ms, max 7.02 ms (budget 16 ms)
Big.java: 50000 lines, 200 keystrokes: p50 2.04 ms, p95 16.88 ms, max 436.46 ms (budget 16 ms)
== new 3
Big.txt: 50000 lines, 200 keystrokes: p50 1.69 ms, p95 3.23 ms, max 8.09 ms (budget 16 ms)
Big.java: 50000 lines, 200 keystrokes: p50 1.96 ms, p95 16.34 ms, max 490.86 ms (budget 16 ms)
== base 4
Big.txt: 50000 lines, 200 keystrokes: p50 1.98 ms, p95 3.85 ms, max 9.26 ms (budget 16 ms)
Big.java: 50000 lines, 200 keystrokes: p50 2.08 ms, p95 15.21 ms, max 408.68 ms (budget 16 ms)
== new 4
Big.txt: 50000 lines, 200 keystrokes: p50 2.02 ms, p95 3.38 ms, max 17.52 ms (budget 16 ms)
Big.java: 50000 lines, 200 keystrokes: p50 1.93 ms, p95 15.11 ms, max 385.54 ms (budget 16 ms)
== new 5
Big.txt: 50000 lines, 200 keystrokes: p50 1.73 ms, p95 3.03 ms, max 18.47 ms (budget 16 ms)
Big.java: 50000 lines, 200 keystrokes: p50 2.22 ms, p95 19.27 ms, max 410.43 ms (budget 16 ms)
== base 5
Big.txt: 50000 lines, 200 keystrokes: p50 1.97 ms, p95 2.97 ms, max 4.92 ms (budget 16 ms)
Big.java: 50000 lines, 200 keystrokes: p50 2.02 ms, p95 14.72 ms, max 407.14 ms (budget 16 ms)
== new 6
Big.txt: 50000 lines, 200 keystrokes: p50 2.13 ms, p95 3.68 ms, max 6.91 ms (budget 16 ms)
Big.java: 50000 lines, 200 keystrokes: p50 2.11 ms, p95 15.99 ms, max 430.13 ms (budget 16 ms)
== base 6
Big.txt: 50000 lines, 200 keystrokes: p50 1.91 ms, p95 3.08 ms, max 8.90 ms (budget 16 ms)
Big.java: 50000 lines, 200 keystrokes: p50 2.02 ms, p95 15.29 ms, max 464.00 ms (budget 16 ms)
== new 7
Big.txt: 50000 lines, 200 keystrokes: p50 1.69 ms, p95 3.07 ms, max 5.06 ms (budget 16 ms)
Big.java: 50000 lines, 200 keystrokes: p50 2.09 ms, p95 15.22 ms, max 453.34 ms (budget 16 ms)
== base 7
Big.txt: 50000 lines, 200 keystrokes: p50 2.34 ms, p95 3.30 ms, max 4.89 ms (budget 16 ms)
Big.java: 50000 lines, 200 keystrokes: p50 2.13 ms, p95 15.54 ms, max 418.04 ms (budget 16 ms)
== new 8
Big.txt: 50000 lines, 200 keystrokes: p50 1.68 ms, p95 3.18 ms, max 7.35 ms (budget 16 ms)
Big.java: 50000 lines, 200 keystrokes: p50 2.02 ms, p95 16.19 ms, max 504.58 ms (budget 16 ms)
== base 8
Big.txt: 50000 lines, 200 keystrokes: p50 1.84 ms, p95 3.99 ms, max 54.60 ms (budget 16 ms)
Big.java: 50000 lines, 200 keystrokes: p50 2.00 ms, p95 19.27 ms, max 514.26 ms (budget 16 ms)
== new 9
Big.txt: 50000 lines, 200 keystrokes: p50 1.64 ms, p95 2.96 ms, max 4.27 ms (budget 16 ms)
Big.java: 50000 lines, 200 keystrokes: p50 1.98 ms, p95 14.82 ms, max 433.45 ms (budget 16 ms)
== base 9
Big.txt: 50000 lines, 200 keystrokes: p50 2.06 ms, p95 2.88 ms, max 3.82 ms (budget 16 ms)
Big.java: 50000 lines, 200 keystrokes: p50 1.98 ms, p95 16.14 ms, max 560.64 ms (budget 16 ms)
== new 10
Big.txt: 50000 lines, 200 keystrokes: p50 1.79 ms, p95 3.75 ms, max 6.42 ms (budget 16 ms)
Big.java: 50000 lines, 200 keystrokes: p50 2.09 ms, p95 14.98 ms, max 428.67 ms (budget 16 ms)
== base 10
Big.txt: 50000 lines, 200 keystrokes: p50 2.13 ms, p95 3.21 ms, max 11.68 ms (budget 16 ms)
Big.java: 50000 lines, 200 keystrokes: p50 1.98 ms, p95 17.72 ms, max 410.88 ms (budget 16 ms)
```

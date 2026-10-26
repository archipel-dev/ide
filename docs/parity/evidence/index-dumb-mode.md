# Indexing, dumb mode and persistence (PLAT-009, IJ-NAV-080)

Evidence for the `archipel-ide-index` package (WP2). Everything below is reproducible from
this repository; nothing is claimed as verified against the reference IDE.

## What is covered

| Behavior | Test |
|---|---|
| Both storage backends satisfy one contract (put/get/keys/forward, replace, remove, hashes, completeness, version bump, file table) | `crates/archipel-ide-index/tests/storage.rs` `memory_store_conforms`, `redb_store_conforms` (`conformance::run`) |
| An index survives a restart | `storage::data_survives_a_restart`, `incremental::the_index_survives_a_restart_and_only_changed_files_are_reindexed` (only the changed and the new file are indexed; file ids are stable; third session indexes nothing) |
| Truncated or garbage index files are rebuilt without error; an open file is never deleted by a second opener | `storage::a_truncated_index_file_is_rebuilt_without_error`, `a_garbage_index_file_is_rebuilt_without_error`, `a_garbage_file_table_resets_the_whole_store`, `an_open_index_file_is_never_deleted_by_a_second_opener`, `incremental::an_unusable_store_directory_falls_back_to_memory` |
| A version bump rebuilds only that index | `storage::a_version_bump_rebuilds_only_that_index` |
| Incremental index equals a fresh full index after random edit / rename / delete / directory-rename sequences (ids kept across explicit moves) | `incremental::incremental_index_equals_a_fresh_full_index`, `incremental_index_with_explicit_moves_equals_a_fresh_full_index` (proptest, 48 cases each, all indexes compared key by key) |
| Same-length rewrite inside one mtime tick is picked up | `incremental::same_length_rewrite_within_one_mtime_tick_is_picked_up` |
| Dumb mode is per index: `require` returns `IndexNotReady` until `Ready`, names are ready before contents, trigrams are a second pass | `builtin::dumb_mode_is_per_index_and_require_returns_index_not_ready`, `trigrams_are_built_in_a_second_pass_after_navigation_is_ready` |
| Open buffers overlay the persistent entries; closing reconciles with the disk | `builtin::open_buffers_overlay_the_persistent_index` |
| TODO pattern change rebuilds only `Todo` | `builtin::changing_todo_patterns_rebuilds_only_the_todo_index`, `todo_commands::tests::rows_follow_scope_filter_and_settings` |
| The threaded executor gives the same result as the inline one | `incremental::the_threaded_executor_gives_the_same_result` |

## Benchmark

`cargo bench -p archipel-ide-index --bench index` generates 20,000 files (Java, Rust and text, 35 MiB
in 40 module directories), indexes them into a `redb` store with the real executor (3 workers), then
restarts on the same store. Container with 4 shared cores (other agents were running), release build:

| Measurement | Result | Spec 1.6 target |
|---|---|---|
| scan + `FileName`/`FileType` ready (Go to File works) | 0.29 s | within 1 s |
| Stubs (all four stub indexes), Words, Todo ready | 17.4 s for 20,000 files | 30 s for 10,000 files |
| Trigram ready (second pass, does not block navigation) | 92 s | not in the spec |
| resident memory after the cold run | 155 MiB | < 200 MiB |
| Go to File query (7 hits of 20,000 names) | 5.8 ms | |
| all symbols (174,000), by name, word lookup, trigram candidates, subtypes | 448 ms, 28 us, 38 ms, 51 ms, 0.8 ms | |
| restart on the same project: `FileName` ready / idle, files reindexed | 29 ms / 101 ms, 0 | |
| store size on disk | 753 MiB (nearly all `Trigram`: one row per trigram and file) | |

The trigram store is the open performance item: one B-tree row per (trigram, file) is simple and
incremental but large. A chunked posting layout (one row per trigram and 4096-file chunk) is the
planned optimization; `Trigram` is text-only, so it never keeps the IDE in dumb mode and Find in
Files falls back to a text scan until it is ready.

## UI

Status bar: spinner and "Indexing 1,204 / 20,000 files" (hidden for runs of fewer than 10 files),
hover lists every index with its state, right-click offers "Rebuild Indexes". Commands:
`index.status` (JSON with per-index states, progress, skipped files, store kind) and
`index.rebuild` (`{"index": "Words"}` for one index). Scene scripts
(`docs/parity/scenes/30-indexing.json`) use `ARCHIPEL_INDEX_THROTTLE_MS` to hold the indexer in
the busy state while the screenshot is taken.

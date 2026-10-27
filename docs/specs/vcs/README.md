# Version control specification (clean-room)

Requirements for the Archipel IDE version-control stack: local changes and changelists, commit,
Git branches/update/push, the Git log, merge conflicts, stash/shelf/patches, annotate and file
history, the diff viewer, Local History, clone and VCS setup, and (not scheduled) Git hosting.
Written from observed behavior, resource bundles, default keymap and settings sources of IntelliJ
IDEA Community (cited as `ic: ...` in each file). No code, comments or line-by-line paraphrases are
copied; user-visible texts, setting names and defaults are quoted as facts.

## Priority legend

- MUST: needed for a usable Git workflow in the first release.
- SHOULD: expected for parity, scheduled after the MUSTs.
- LATER: specified for completeness or traceability, not scheduled (each LATER row states why).

## Files

| File | Topic | ID prefix | Cases |
|---|---|---|---|
| 01-changes-and-changelists.md | changelists, Local Changes view, unversioned/ignored, file status colors, refresh, gutter markers and popup | VCS-CHG | cases/01-... |
| 02-commit.md | Commit tool window and dialog, checks, amend, author, sign-off, partial commit, staging area | VCS-CMT | cases/02-... |
| 03-branches.md | branches popup, checkout/create/rename/delete, compare/merge/rebase, update, fetch, pull, push, tags, reset, remotes, worktrees | VCS-BR | cases/03-... |
| 04-log.md | Git log: table, graph, filters, details, cherry-pick, revert, reset, in-memory history editing, interactive rebase | VCS-LOG | cases/04-... |
| 05-merge-conflicts.md | conflicts dialog, three-way merge viewer, automatic resolution | VCS-MERGE | cases/05-... |
| 06-stash-and-shelve.md | Git stash, shelf, patches, rollback | VCS-STASH, VCS-SHELF, VCS-PATCH | cases/06-... |
| 07-annotate-and-history.md | blame gutter, file history, selection history, compare with revision | VCS-ANN, VCS-HIST | cases/07-... |
| 08-diff-viewer.md | viewers, highlight and ignore policies, navigation, apply/revert, combined diff, directory diff, external tools | VCS-DIFF | cases/08-... |
| 09-local-history.md | what is recorded, labels, history view, revert, system labels | VCS-LH | cases/09-... |
| 10-clone-and-vcs-setup.md | clone, enabling VCS, mappings, ignore files, tool windows | VCS-SETUP | cases/10-... |
| 11-git-hosting.md | GitHub/GitLab/code-review actions (inventory traceability only) | VCS-HOST | - |

The ID prefix for the commit area is `VCS-CMT` (the grammar of `cargo run -p xtask -- spec-coverage`
accepts at most five letters in the area part, so `VCS-COMMIT` cannot be used).

## Executable cases

`cases/*.cases.md` hold concrete cases in the block format required by the specification brief
(`### <ID> case <n>: <title>`, `settings:`, `language:`, ```before, `keys:`, ```after). For VCS
the `before` block describes repository, working tree, changelists and UI state; the `after` block
lists the resulting state or the visible dialog/list/tree contents in order, with exact message
texts. The notation is defined at the top of `cases/01-changes-and-changelists.cases.md`.

## Requirement counts (by priority)

| Priority | Count |
|---|---|
| MUST | 237 |
| SHOULD | 91 |
| LATER | 22 |

Per file (MUST/SHOULD/LATER): 01 52/15/1; 02 31/19/1; 03 40/13/2; 04 20/8/2; 05 17/4/1;
06 14/4/1 (shelf record row counted MUST); 07 11/9/0; 08 19/8/2; 09 20/3/2; 10 13/8/0; 11 0/0/10.
(Counted from the requirement lines of the tables; the generated `docs/specs/coverage.md` is the
authority.) Cases: 216.

## Inventory coverage

Every action, settings page, tool window and status-bar widget listed in
`docs/specs/inventory/{git,git-hosting,diff-viewer,local-history,vcs-core,vcs-log}.md` (on branch
`docs/feature-inventory`) appears with its ID in backticks in at least one requirement of these
files. Items that are internal, diagnostic, network-bound or belong to another area are marked
LATER with the reason in the row (VCS-BR-078/079, VCS-LOG-090, VCS-DIFF-072/073, VCS-LH-053/054,
VCS-HOST-*).

## Related specs

Saving documents before VCS operations: `PLAT-SAVE-040..045`
(`docs/specs/platform/09-saving-and-autosave.md`, not present on this branch). Virtual file events
that drive the dirty-scope refresh: `PLAT-VFS-*` (`docs/specs/platform/01-vfs-and-documents.md`).

## Open questions (top)

- Pixel metrics and colors that the sparse source does not contain must be measured with
  `tools/reference/`: new-UI branches popup, log table and graph, diff band colors and connectors,
  annotation column widths, hover width of the gutter bar.
- Defaults not found in the sparse clone: advanced settings `Open Diff as Editor Tab` and
  `All-in-One Diff`; editor settings page hosting the gutter-marker toggles
  (`SHOW_LST_GUTTER_MARKERS`).
- Light scheme defines the ignored file color twice (`7d5020` and `8C4F00`); confirm the effective one.
- Whether `Analyze code` and `Check TODO` block the commit by default in the non-modal UI (the
  source marks both as post-commit checks; the postpone setting is on by default).
- Composition of choice-format confirmation texts (merge apply dialog) was derived by hand.
- Several UI composition details depend on provider extensions (annotation popup entries, VCS
  operations popup, history views); marked OQ in their files.

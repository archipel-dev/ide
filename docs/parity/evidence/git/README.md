# Git evidence

Captured on Linux under Xvfb (1600×1000, `ARCHIPEL_RENDERER=glow`,
`ARCHIPEL_NO_UPDATE_CHECK=1`) with a `java-gradle` sample repository whose
`App.java` matches the reference scene: one commit, then the line
`// Greets each build tool by name` inserted before the `for` loop.

| File | Steps | Reference scene |
|---|---|---|
| `13-diff-side-by-side.png` | Alt+0, double-click `App.java` in the Commit window | 13 Diff |
| `13b-diff-external-change-words.png` | With the diff open, run `sed -i 's/"Hello, "/"Hi there, "/' App.java` and `echo new > NOTES.txt` in another process; wait ~2 s, no clicks | 12 Commit / 13 Diff |
| `13c-diff-unified.png` | F7, then choose “Unified viewer” in the toolbar | 13 Diff (unified mode) |
| `12c-tree-autorefresh.png` | Change `settings.gradle` outside the IDE, Alt+1 | issue #47 bug 3 |
| `14-git-log.png` | Sample history with a merged `feature/greeting` branch and tag `v1.0`; Alt+9 | 14 Git log |
| `14b-git-log-commit-diff.png` | Select the merge commit, double-click `settings.gradle` in the changed files | 14 Git log / 13 Diff |
| `14c-git-log-filter.png` | Type `greet` in “Text or hash” | 14 Git log |
| `branches-popup.png` | Click the `master` VCS widget | branches popup (no numbered scene) |
| `branches-popup-actions.png` | Hover `feature/greeting` | branch submenu |
| `branches-new-branch.png` | After Checkout of `feature/greeting` (widget and tree updated), New Branch… and type `feature/login`; Enter creates and checks it out | — |
| `push-dialog.png` | Sample with a bare `origin`; two local commits; Ctrl+Shift+K | Push dialog |
| `update-project-commit-and-push.png` | Push, then push a change from another clone; Ctrl+T updates the project and the open `App.java` shows the pulled change (`"Hey, "`); an outside edit of `settings.gradle`, Alt+0 shows the Commit window with “Commit and Push…” | Update Project / Commit and Push |
| `editor-git-context-menu.png` | Right-click in the editor, hover Git | editor context menu Git submenu |
| `blame-and-file-history.png` | Git ▸ Annotate with Git Blame, then Git ▸ Show History, hover the annotation of line 7 | Annotate / Show History |
| `12d-commit-toolbar-amend.png` | Outside edit of `settings.gradle` plus an unversioned `TODO.txt`; Alt+0 | 12 Commit (toolbar row, Amend, “1 modified”) |
| `commit-rollback-dialog.png` | Tick Amend (last message loaded, button becomes “Amend Commit”), then the Rollback toolbar button | 12 Commit |
| `unstash-dialog.png` | Branches popup ▸ Stash Changes… “WIP build tweak”, then Unstash Changes… | Stash |
| `gutter-change-markers.png` | Open `App.java` with a modified line (6), an inserted comment (11) and a deleted `import` (wedge above 3) | editor gutter (no numbered scene) |
| `gutter-change-popup.png` | Click the blue marker on line 6 | gutter change popup |
| `gutter-rollback.png` | Click “Rollback” in the popup: line 6 is back to the HEAD text and its marker is gone | — |

Compared with reference scene 13 (IntelliJ IDEA, `java-gradle`):

- Same structure: editor tab for the diff, toolbar with previous/next arrows and a
  viewer dropdown, “N difference(s)” on the right, base revision hash and path
  above the left pane, “Current version” above the right pane, two line-number
  columns with the connector between them, green inserted block, error-stripe mark.
- Both panes use the editor's tree-sitter highlighting (Java here).
- Differences: no whitespace/highlighting dropdowns, settings gear or help, no include-in-commit
  checkbox in the gutter, the tab is titled `App.java (Diff)` rather than
  `Commit`, and the diff is read-only.

Background refresh: `git --no-optional-locks status` every 2 s on a worker
thread and immediately when the window regains focus, so it never takes
`index.lock` away from the user's own git commands.

Compared with reference scene 14 (IntelliJ IDEA, `java-gradle`): same three-pane
split (branches, commit table, changed files over commit details), the
“Text or hash” field, a Branch filter, ref labels with branch/tag icons, and a
`hash author <email> on date` details line. Differences: no `Log` tab strip or
`+` for more log tabs, no User/Date/Paths filters or toolbar actions, branches
pane has no search or action strip, the changed files are a flat list (not a
directory tree), dates are UTC.

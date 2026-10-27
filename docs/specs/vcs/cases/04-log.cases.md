# Cases for 04 (Git log)

Notation as in `01-changes-and-changelists.cases.md`.

### VCS-LOG-002 case 1: empty repository
settings: default
language: plain
```before
repo: no commits
```
keys: action Vcs.Show.Log
```after
table placeholder "No changes committed." with link "Commit local changes"
```

### VCS-LOG-002 case 2: filter without match
settings: default
language: plain
```before
repo with commits; text filter "zzzz"
```
keys: Enter in the filter
```after
placeholder "No commits matching filters" with link "Reset filters"
```

### VCS-LOG-003 case 1: default columns
settings: default
language: plain
```before
single-root repo
```
keys: (open log)
```after
columns left to right: Subject (graph, refs, message), Author, Date; no Hash column, no Root column
reference chips show only the first reference of a commit (compact view)
```

### VCS-LOG-004 case 1: go to parent
settings: default
language: plain
```before
merge commit m (parents p1, p2) selected
```
keys: Right
```after
popup "Select Parent to Navigate" listing two rows in the form "<hash> <subject> by <author> on <date> at <time>"
```

### VCS-LOG-004 case 2: go to hash errors
settings: default
language: plain
```before
log open
```
keys: Ctrl+F, type "xyz", Enter
```after
popup label "Enter hash or branch/tag name:"; message "'xyz' does not look like a commit hash"
```

### VCS-LOG-004 case 3: commit not in view
settings: branch filter = feature
language: plain
```before
commit c1 only on main
```
keys: Ctrl+F, type hash of c1, Enter
```after
message "c1... doesn't match the filters" (hash prefix shown) with links "View and reset filters", "View in new tab"
```

### VCS-LOG-010 case 1: text filter
settings: default
language: plain
```before
commits: "fix login", "add search", "fix typo"
```
keys: Ctrl+L, type "fix", Enter
```after
rows: "fix typo", "fix login" (newest first); the text field keeps "fix"
```

### VCS-LOG-011 case 1: branch filter popup
settings: default
language: plain
```before
branches main, feature; HEAD on main
```
keys: click Branch filter
```after
popup rows in order: "All", "HEAD (Current Branch)", "Favorites", "Select...", section "Local": main, feature; section "Remote"; section "Tags"
```

### VCS-LOG-013 case 1: date filter presets
settings: default
language: plain
```before
log open
```
keys: click Date filter
```after
popup rows: "Last 24 hours", "Last 7 days", "Select..."
choosing "Last 7 days" shows button text "Last 7 days" or "since <date>"
```

### VCS-LOG-014 case 1: path filter
settings: default
language: plain
```before
commit c1 touches src/A.java, c2 touches docs/r.md
```
keys: Paths > choose "src"
```after
only c1 listed; presentation "for src"
```

### VCS-LOG-020 case 1: details panel text
settings: default
language: plain
```before
commit by "Ann <ann@x>" authored 2024-01-02 10:00, committed by "Bob" 2024-01-03 11:00, message "Subject\n\nBody"
```
keys: select commit
```after
details: "Subject" bold, "Body", hash, "Ann on <date> at <time>" and "committed by Bob on <date> at <time>"
branches line: "In branches: loading..." then "In 1 branch: main"
```

### VCS-LOG-022 case 1: context menu order
settings: default
language: plain
```before
single non-merge commit selected in the log, repository clean
```
keys: right click
```after
menu top to bottom: Copy Revision Number, Create Patch..., Cherry-Pick, Compare with Local, Show Repository at Revision, Checkout (submenu), New Branch..., New Tag..., separator, Reset Current Branch to Here..., Revert Commit, Undo Commit... (only last commit), separator, Edit Commit Message..., Fixup..., Squash Into..., Drop Commits, Interactively Rebase from Here..., Push All up to Here..., then Go to Child Commit, Go to Parent Commit
```

### VCS-LOG-030 case 1: cherry-pick
settings: default
language: plain
```before
main: m1; feature: m1 + f1 "Add F" (adds F.txt); HEAD on main
```
keys: select f1 > Cherry-Pick
```after
main: m1 + f1' "Add F" (new hash); F.txt exists; notification "Cherry-pick successful"
```

### VCS-LOG-030 case 2: cherry-pick conflict
settings: default
language: plain
```before
main and f1 both change line 1 of A.java differently
```
keys: Cherry-Pick f1
```after
notification "Cherry-pick was performed with conflicts" (title) with text "There are unresolved conflicts in the working tree." actions "Resolve...", "Abort Cherry-Pick"; conflicts dialog header "Conflicts during cherry-picking commit <h> made by <author>"
```

### VCS-LOG-032 case 1: revert
settings: default
language: plain
```before
HEAD r2 "Break it"
```
keys: select r2 > Revert Commit
```after
new commit on top with subject starting "Revert \"Break it\""; tree equals the tree before r2
```

### VCS-LOG-032 case 2: revert merge
settings: default
language: plain
```before
merge commit selected
```
keys: open menu
```after
"Revert Commit" disabled with description "Reverting merge commits is not allowed"
```

### VCS-LOG-033 case 1: undo commit
settings: default
language: plain
```before
HEAD c2 "Last" with file X.java; lists Changes*, Other
```
keys: select c2 > Undo Commit..., choose list "Changes"
```after
HEAD c1; X.java modified in list "Changes"; notification "Undo commit successful"
```

### VCS-LOG-033 case 2: not last commit
settings: default
language: plain
```before
select c1 (not the last)
```
keys: Undo Commit...
```after
disabled; description "The selected commit is not the last in the current branch"
```

### VCS-LOG-034 case 1: reword
settings: default
language: plain
```before
history: c3 "Third", c2 "Second", c1 "First"; select c2
```
keys: F2, replace text with "Second v2", Enter
```after
dialog title "Edit Commit Message", label "Edit message for commit <hash> by <author>"
history: c3' "Third", c2' "Second v2", c1 "First"
notification "Commit message changed" with action "Undo"
```

### VCS-LOG-034 case 2: pushed to protected branch
settings: default
language: plain
```before
c2 is on origin/main (protected)
```
keys: F2
```after
disabled/failed with text "The commit is already pushed to protected branch 'origin/main'"
```

### VCS-LOG-035 case 1: squash
settings: default
language: plain
```before
c3 "C", c2 "B", c1 "A" ; select c3 and c2
```
keys: Squash Commits...
```after
dialog "Squashed Commit Message" label "Edit message for squashed commit"; after OK with "BC": history c23 "BC", c1 "A"; notification "Commits Squashed" with "Undo"
```

### VCS-LOG-035 case 2: drop confirmation
settings: Ask for confirmation to drop commits = on
language: plain
```before
select c2 on branch main
```
keys: Drop Commits
```after
dialog "Drop Commit": "Do you want to drop 1 commit from the main branch?"; after OK notification "Dropped 1 commit"
```

### VCS-LOG-036 case 1: interactive rebase dialog
settings: default
language: plain
```before
c3 "C", c2 "B", c1 "A"; invoke "Interactively Rebase from Here..." on c1
```
keys: open
```after
dialog "Rebasing Commits": table columns "Action", "Hash", "Subject"; rows (top oldest first): Pick c2 "B", Pick c3 "C"; actions per row: Pick, Reword, Edit, Squash, Fixup, Drop; link "View Git Commands", link "Reset", button "Start Rebasing"
```

### VCS-LOG-036 case 2: drop via dialog
settings: default
language: plain
```before
rebasing dialog as above
```
keys: set c2 to Drop, click "Start Rebasing"
```after
history: c3' "C", c1 "A"; notification "Rebase successful"
```

### VCS-LOG-040 case 1: file history toolbar
settings: default
language: plain
```before
file A.java with 3 commits
```
keys: action Vcs.ShowTabbedFileHistory
```after
tab "History"; rows: 3 commits (newest first); toolbar: Refresh, Show Diff, Show All Affected Files, view options, ...
Ctrl+D on one row opens the diff of that commit against its parent
```

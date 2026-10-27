# Cases for 03 (branches, update, push, tags, remotes)

Notation as in `01-changes-and-changelists.cases.md`.

### VCS-BR-004 case 1: popup content and order
settings: default (new UI)
language: plain
```before
repo: main (a2 "Two", a1 "One"); local: main, feature/x, fix; remote: origin/main, origin/dev; tags: v1
```
keys: Ctrl+Shift+Backquote
```after
popup rows top to bottom:
  search field, placeholder "Search for branches, actions"
  "Update Project..." (Ctrl+T), "Commit..." (Ctrl+K), "Push..." (Ctrl+Shift+K)
  separator
  "New Branch..." (Ctrl+Alt+N), "New Worktree...", "Checkout Tag or Revision..."
  separator
  section "Recent": (empty when none)
  section "Local": main (current marker), fix, folder "feature" > x
  section "Remote": origin > main, dev
  section "Tags": v1
```

### VCS-BR-012 case 1: search with no result
settings: default
language: plain
```before
popup open
```
keys: type "zzz"
```after
single row text "Branch not found"
```

### VCS-BR-013 case 1: local branch submenu
settings: default
language: plain
```before
popup open, branch "fix" selected (current is main)
```
keys: Enter
```after
submenu items in order:
  "Checkout"
  "New Branch from 'fix'..."
  "Checkout and Rebase onto 'main'"
  separator
  "Compare with 'main'"
  "Show Diff with Working Tree"
  separator
  "Rebase 'main' onto 'fix'"
  "Merge 'fix' into 'main'"
  separator
  "New Worktree..."
  separator
  "Update"
  "Push..."
  "Pull into 'main' Using Rebase"
  "Pull into 'main' Using Merge"
  separator
  "Rename..." (F2)
  "Delete" (Delete)
```

### VCS-BR-020 case 1: new branch dialog
settings: default
language: plain
```before
repo on main with commits
```
keys: Ctrl+Alt+N
```after
dialog titled "Create New Branch":
  "Branch Name:" field, focused, prefilled with the previous default if any
  checkbox "Checkout branch" (checked)
  buttons: "Create", "Cancel"
```

### VCS-BR-021 case 1: validation messages
settings: default
language: plain
```before
local branches: main, feature; remote: origin/dev; dialog open
```
keys: type each value in turn
```after
"" -> "Specify name for the new branch"
"a..b" -> "Branch name a..b is not valid"
"HEAD" -> "HEAD is a reserved keyword"
"main" -> "Branch name main already exists" and checkbox "Overwrite existing branch" appears with "Change the name or overwrite existing branch"
"dev" -> "Branch name dev conflicts with remote branch with the same name"
"feature/x" -> "Branch name feature/x conflicts with local branch directory with the same name"
"ok-name" -> no message, "Create" enabled
```

### VCS-BR-022 case 1: create and checkout
settings: default
language: plain
```before
repo: main (a1)
```
keys: Ctrl+Alt+N, type "topic", Enter
```after
repo: topic (a1) checked out; branch list adds topic
notification: "Checked out new branch topic from main"
```

### VCS-BR-023 case 1: checkout remote branch
settings: default
language: plain
```before
remote: origin/dev (d1); no local dev
```
keys: popup > Remote > origin/dev > "Checkout"
```after
local branch dev created at d1 tracking origin/dev, checked out
notification "Checked out dev"
```

### VCS-BR-024 case 1: smart checkout dialog
settings: save policy Shelve (default)
language: plain
```before
repo: main; branch other differs in A.java; work: M A.java
```
keys: popup > other > Checkout
```after
dialog titled "Git Checkout Problem"; text "Your local changes to the following files would be overwritten by checkout." and the IDE shelve sentence; file list: A.java; buttons "Smart Checkout", "Force Checkout", "Cancel"
after Smart Checkout: other checked out and A.java modification restored
```

### VCS-BR-026 case 1: checkout tag or revision
settings: default
language: plain
```before
tags: v1 at a1; HEAD at a2
```
keys: popup > "Checkout Tag or Revision...", type "v1", Enter
```after
prompt text "Enter reference (branch, tag) name or commit hash:"
HEAD detached at a1; widget text shows "v1" or the hash; status "Git: Detached HEAD doesn't point to any branch" in tooltip
```

### VCS-BR-028 case 1: rename
settings: default
language: plain
```before
local: main, old
```
keys: select "old", F2, type "new", Enter
```after
dialog titled "Rename Branch old"; branch list: main, new
notification "Branch old was renamed to new"
```

### VCS-BR-029 case 1: delete unmerged
settings: default
language: plain
```before
branch topic has commit t1 not in main; current branch main
```
keys: select topic, Delete
```after
dialog "Branch Was Not Fully Merged": "The branch topic was not fully merged to main." "Below is the list of unmerged commits." list: t1; buttons "Restore", delete-anyway
```

### VCS-BR-029 case 2: delete merged with restore
settings: default
language: plain
```before
branch done fully merged
```
keys: select done, Delete, then click notification action "Restore"
```after
after delete: notification "Deleted branch done" with actions "Restore", "View Commits", "Delete Tracked Branch" (last only if tracked)
after Restore: branch done exists again at its old commit
```

### VCS-BR-040 case 1: compare with current
settings: default
language: plain
```before
main: m1; feature: m1 + f1 (one extra); popup on feature
```
keys: "Compare with 'main'"
```after
tab titled "Compare" with suffix "feature and main"; log pane header "Commits that exist in feature but don't exist in main"; one row f1
swapping shows "main contains all commits from feature"
```

### VCS-BR-042 case 1: merge dialog options
settings: default
language: plain
```before
repo main; branch feature
```
keys: action Git.Merge, choose "feature", click "Modify options" > "--no-ff"
```after
dialog title "Merge Changes"; option chip "--no-ff" shown; popup items before choosing:
  "--no-ff  Create a merge commit even if it can be fast-forwarded"
  "--ff-only  Merge only if it can be fast-forwarded"
  "--squash  Create a single commit for all merged changes"
  "-m  Specify a message for the merge commit"
  "--no-commit  Do not commit the merge result"
  "--no-verify  Bypass the pre-merge and commit message hooks"
  "--allow-unrelated-histories  Allow merging histories that do not share a common ancestor."
result: merge commit created even if fast-forward possible
```

### VCS-BR-042 case 2: merge dialog errors
settings: default
language: plain
```before
merge dialog open
```
keys: click Merge with empty branch
```after
error "The merge branch should be selected."
```

### VCS-BR-043 case 1: already merged
settings: default
language: plain
```before
feature fully merged into main
```
keys: Merge feature
```after
notification "Already up to date"
```

### VCS-BR-044 case 1: rebase dialog validation
settings: default
language: plain
```before
dialog Git.Rebase
```
keys: click Rebase with empty upstream
```after
error "Select an upstream"
```

### VCS-BR-045 case 1: conflict during rebase
settings: default
language: plain
```before
main and feature both change line 1 of A.java differently
```
keys: Rebase feature onto main
```after
notification titled "Rebase stopped due to conflicts" with actions "Resolve...", "Continue", "Abort"
conflicts dialog opens listing A.java with type "both modified"
```

### VCS-BR-045 case 2: published commits
settings: default
language: plain
```before
rebase of commits already pushed to protected branch main
```
keys: Rebase
```after
dialog titled "Rebasing Published Commit": "You are trying to rebase some commits already pushed to a protected branch." / "Rebasing them would duplicate commits, which is not recommended and most likely unwanted."; buttons "Rebase Anyway", "Cancel"
```

### VCS-BR-050 case 1: update project merge
settings: Update method Merge, save policy Shelve
language: plain
```before
main tracks origin/main; origin/main has 2 new commits changing 3 files; work clean
```
keys: Ctrl+T
```after
HEAD is a merge-free fast-forward to origin/main (no local commits)
notification: "3 Files Updated" with text "3 files updated in 2 commits" and action "View Commits"
```

### VCS-BR-050 case 2: detached HEAD
settings: default
language: plain
```before
HEAD detached
```
keys: Ctrl+T
```after
notification "Cannot Update: No Current Branch" text starts "You are in the 'detached HEAD' state, which means that you're not on any branch"
```

### VCS-BR-051 case 1: local changes restored
settings: save policy Shelve
language: plain
```before
work: M A.java (not touched upstream); origin/main has new commit touching B.java
```
keys: Ctrl+T
```after
A.java is again modified after update; no leftover shelf entry; notification mentions update success
```

### VCS-BR-053 case 1: fetch prune note
settings: default
language: plain
```before
origin/gone exists locally but was deleted on the server
```
keys: action Git.Fetch
```after
remote-tracking branch origin/gone removed; notification "Fetch successful" and text "Pruned obsolete remote reference: origin/gone"
```

### VCS-BR-054 case 1: pull dialog
settings: default
language: plain
```before
repo with remote origin
```
keys: action Git.Pull
```after
dialog titled "Pull to main" with "Remote" and branch field placeholder "Specify branch"; option chips menu "Add Pull Options" containing:
  "Merge only if it can be fast-forwarded"
  "Create a merge commit even if it can be fast-forwarded"
  "Create a single commit for all pulled changes"
  "Merge, but do not commit the result"
  "Rebase the current branch on top of incoming changes"
button "Pull"
```

### VCS-BR-060 case 1: push dialog tree
settings: default
language: plain
```before
main tracks origin/main; local commits p2 "Second", p1 "First" unpushed
```
keys: Ctrl+Shift+K
```after
dialog titled "Push Commits to origin:main" (form "Push Commits to <remote>:<branch>")
tree: repository node > "main -> origin/main" > commits "Second", "First"
button "Push" (default), arrow menu with "Force Push"
```

### VCS-BR-061 case 1: new target
settings: default
language: plain
```before
branch topic without upstream; remote origin
```
keys: Ctrl+Shift+K
```after
target shown as "origin/topic" with marker "New"; checkbox "Set upstream" available
```

### VCS-BR-062 case 1: force push to protected
settings: protected patterns master, main
language: plain
```before
current branch main
```
keys: Push dialog > arrow > "Force Push"
```after
refused with text "Force push to origin/main is prohibited" and link "Configure protected branches in '<settings path>'"
```

### VCS-BR-062 case 2: force push confirmation
settings: advanced vcs.allow.force.push.without.confirmation=false
language: plain
```before
branch topic
```
keys: Force Push
```after
dialog "Force Push": "You're going to force push to origin/topic. It may overwrite commits at the remote. Are you sure you want to proceed?" checkbox "Don't warn about this target"
```

### VCS-BR-063 case 1: push notifications
settings: default
language: plain
```before
two commits pushed successfully to origin/main
```
keys: Push
```after
notification title "Push successful", text "Pushed 2 commits to origin/main"
```

### VCS-BR-064 case 1: rejected push
settings: default
language: plain
```before
remote has new commit; local has one
```
keys: Push
```after
notification "Push rejected" with text "Push to origin/main was rejected" and action "Update"
after Update: dialog "Push Rejected": "Push of the current branch "main" was rejected." "Remote changes need to be merged before pushing." buttons "Merge", "Rebase", "Cancel" and checkbox "Remember the update method and update silently in the future."
```

### VCS-BR-070 case 1: new tag dialog
settings: default
language: plain
```before
HEAD a2
```
keys: action Git.Tag
```after
dialog "Tag": "Tag Name:", "Commit:" (tooltip "Enter name of commit or object to tag or leave blank to use HEAD."), "Message:", checkbox "Force", button "Validate", button "Create Tag"
```

### VCS-BR-070 case 2: duplicate tag
settings: default
language: plain
```before
tag v1 exists
```
keys: create tag v1 without Force
```after
error "The tag with the same name exists."
```

### VCS-BR-072 case 1: reset dialog
settings: default
language: plain
```before
HEAD a3; log selection a1; Reset Current Branch to Here...
```
keys: open dialog
```after
dialog "Git Reset": description lines, radio list with descriptions:
  Soft  "Files won't change, differences will be staged for commit."
  Mixed "Files won't change, differences won't be staged." (selected default)
  Hard  "Files will be reverted to the state of the selected commit." + "Warning: any local changes will be lost."
  Keep  "Files will be reverted to the state of the selected commit," "but local changes will be kept intact."
button "Reset"; selecting Hard changes it to "Hard Reset"
result of Mixed: HEAD=a1, index equals a1, worktree unchanged
```

### VCS-BR-073 case 1: remotes dialog validation
settings: default
language: plain
```before
remotes: origin
```
keys: Manage Remotes... > Add, empty name
```after
"Remote name cannot be empty"; name "origin" -> "Remote name 'origin' is already used"; empty URL -> "Remote URL cannot be empty"
```

### VCS-BR-077 case 1: Git settings defaults
settings: default
language: plain
```before
settings page Version Control > Git
```
keys: (open)
```after
Commit group: three warn checkboxes checked, "Warn when committing files larger than" 50 MB
Push group: "Show Push dialog for Commit and Push" checked, "... only when committing to protected branches" unchecked, protected list "master main"
Update group: "Update method:" Merge; "Clean working tree using:" Shelve
"Use credential helper" unchecked; "Enable staging area" unchecked
```

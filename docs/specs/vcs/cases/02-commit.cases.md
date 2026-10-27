# Cases for 02 (commit)

Notation as in `01-changes-and-changelists.cases.md`.

### VCS-CMT-001 case 1: mode selection
settings: default
language: plain
```before
repo: Git only, project window open
```
keys: Ctrl+K
```after
Commit tool window (left stripe) is activated; caret in the message editor (placeholder "Commit Message" visible while empty); no modal dialog
```

### VCS-CMT-001 case 2: modal dialog fallback
settings: system property vcs.force.modal.commit=true
language: plain
```before
repo: Git only
```
keys: Ctrl+K
```after
modal dialog titled "Commit Changes" with tree, "Commit Message" editor, button "Commit"
```

### VCS-CMT-011 case 1: button and shortcut
settings: default
language: plain
```before
work: M A.java
lists: Changes*: A.java (ticked)
ui: Commit tool window, message editor focused, text "Fix A"
```
keys: Ctrl+Enter
```after
commit created on HEAD with subject "Fix A" containing A.java
message editor is empty, tree shows no changes
notification: "1 file committed"
```

### VCS-CMT-012 case 1: disabled reasons
settings: default
language: plain
```before
work: M A.java
ui: A.java unticked, message empty
```
keys: (hover Commit button)
```after
Commit button disabled; status text "Select files to commit and specify commit message"
```

### VCS-CMT-012 case 2: only message missing
settings: default
language: plain
```before
work: M A.java (ticked), message empty
```
keys: (hover)
```after
status "Specify commit message"; button disabled
```

### VCS-CMT-013 case 1: modal empty message
settings: modal mode
language: plain
```before
dialog "Commit Changes" with one ticked file, message empty
```
keys: click "Commit"
```after
message dialog titled "No Commit Message" with text "Add a summary of your changes in the commit message field"; commit dialog stays open
```

### VCS-CMT-014 case 1: message history order
settings: default
language: plain
```before
history: (empty)
```
keys: commit "a", commit "b", commit "a" (three separate commits), then Ctrl+M
```after
popup "Commit Message History" lists, top to bottom: "b", "a"
```

### VCS-CMT-014 case 2: history cap
settings: default
language: plain
```before
history: 25 distinct messages m1..m25 (oldest m1)
```
keys: commit "m26"
```after
history holds m2..m26 (25 entries); m1 is gone
```

### VCS-CMT-016 case 1: subject length inspection
settings: subject limit 72 (default)
language: plain
```before
message editor: 80 x "a" on the first line
```
keys: (hover underline)
```after
first line is underlined; tooltip text "Subject cannot exceed 72 characters"
```

### VCS-CMT-016 case 2: blank line separation
settings: default
language: plain
```before
message: "Subject\nBody starts here"
```
keys: (inspection)
```after
tooltip "Missing blank line between subject and body"; fix "Add blank line" inserts an empty line -> "Subject\n\nBody starts here"
```

### VCS-CMT-020 case 1: partial commit by chunk
settings: default
language: java
```before
HEAD A.java:
line1
line2
line3
line4
line5
work A.java: line1 -> L1, line3 -> L3, line5 -> L5 (three separate chunks)
ui: diff of A.java in Commit window; only chunk 2 (line3) ticked
message: "partial"
```
keys: Ctrl+Enter
```after
HEAD commit diff changes only line3 (line3 -> L3)
working file still contains L1 and L5 as local modifications
tree after commit: A.java (modified) with chunks 1 and 5 remaining
```

### VCS-CMT-022 case 1: tri-state checkbox label
settings: default
language: java
```before
A.java with 3 chunks, 1 included
```
keys: (tree rendering)
```after
A.java node checkbox is in the mixed state; secondary text "1 of 3 changes"
```

### VCS-CMT-023 case 1: partial commit not supported
settings: default
language: plain
```before
binary file img.png modified; checkbox in mixed state is forced via a split
```
keys: Commit
```after
dialog "Partial Commit Is Not Supported" with text "Partial commit is not supported for 'img.png'." newline "All changes in the selected files will be committed."
```

### VCS-CMT-021 case 1: multiple changelists
settings: default
language: plain
```before
lists: Changes*: A.java; Fix: B.java; both files ticked; message "x"
```
keys: Ctrl+Enter
```after
commit is not made; check failure shown with text "Multiple changelists are selected for commit"
```

### VCS-CMT-030 case 1: options popup contents
settings: default
language: plain
```before
ui: Commit window
```
keys: Ctrl+O
```after
popup rows in order:
  "Author:" text field
  checkbox "Amend"
  checkbox "Sign-off commit" (unchecked)
  checkbox "Run Git hooks" (checked)
  checkbox "Create extra commit with file movements" (checked, disabled when no renames)
  group "Commit Checks": Reformat code, Rearrange code, Optimize imports, Cleanup, Analyze code, Check TODO
  group "Advanced Commit Checks" (when post-commit checks exist)
```

### VCS-CMT-031 case 1: amend toggle
settings: default
language: plain
```before
repo: main (a2 "Add feature", a1 "Initial")
work: M A.java
```
keys: Alt+M
```after
message editor text: "Add feature"
Commit button text: "Amend Commit"
tree gains the amended commit's files under "Amended commit" while loading text "Loading amended commit..." disappears
```

### VCS-CMT-031 case 2: amend commit result
settings: default
language: plain
```before
repo: main (a2 "Add feature", a1 "Initial"), work: M A.java (ticked), amend on, message "Add feature v2"
```
keys: Ctrl+Enter
```after
repo: main (a2' "Add feature v2", a1 "Initial") where a2' contains the old a2 changes plus A.java
amend toggle is off again
```

### VCS-CMT-033 case 1: custom author
settings: default
language: plain
```before
git config user.name "Dev One", user.email dev1@x
author field text "Ann <ann@x>"; message "m"; one file ticked
```
keys: Ctrl+Enter
```after
commit author "Ann <ann@x>", committer "Dev One <dev1@x>"
author list history now begins with "Ann <ann@x>" (max 16 entries)
```

### VCS-CMT-034 case 1: sign-off
settings: Sign-off commit on
language: plain
```before
git config user.name "Dev One", user.email dev1@x, message "Fix"
```
keys: Ctrl+Enter
```after
commit message "Fix\n\nSigned-off-by: Dev One <dev1@x>"
```

### VCS-CMT-041 case 1: failed check notification
settings: default (post-commit slow checks on); Analyze code on but forced to LATE by turning the setting off
language: java
```before
file A.java contains a compile error; "Run advanced checks after a commit is done" = off
```
keys: Ctrl+Enter
```after
no commit; notification titled "Commit checks failed" with actions "Commit Anyway", "Show details"
```

### VCS-CMT-041 case 2: modal failure dialog
settings: modal mode
language: java
```before
same file; dialog "Commit Changes"
```
keys: click "Commit"
```after
dialog titled "Commit Check Failed"; buttons in order: "Show Details", "Commit", "Cancel"
```

### VCS-CMT-044 case 1: post-commit checks do not block
settings: default
language: java
```before
file with a TODO added in the change; Check TODO on
```
keys: Ctrl+Enter
```after
commit is created first; afterwards notification "Commit contains problems" with action "Ignore"
```

### VCS-CMT-046 case 1: analysis dialog text
settings: Analyze code on, post-commit off
language: java
```before
one error and two warnings in the changed file
```
keys: Ctrl+Enter
```after
dialog text: "File A.java contains problems." / "One error and 2 warnings found." / "Would you like to review them?"
buttons: "Review code analysis", "Commit Anyway"-style commit button, "Cancel"
```

### VCS-CMT-047 case 1: unresolved conflict
settings: default
language: plain
```before
merge in progress, conflicted file C.java ticked
```
keys: Ctrl+Enter
```after
commit refused with text "Cannot commit changes due to unresolved conflicts."
```

### VCS-CMT-050 case 1: missing identity
settings: default
language: plain
```before
git config user.name/user.email unset (local and global)
```
keys: Ctrl+Enter
```after
dialog titled "Git User Name Is Not Defined":
  "Name:" field, "E-mail:" field, checkbox "Set properties globally" (checked)
  buttons: "Set and Commit", "Cancel"
entering email "bad" shows error "The email address must contain the '@' symbol"
```

### VCS-CMT-051 case 1: CRLF warning
settings: Warn if CRLF line separators are about to be committed = on
language: plain
```before
changed file with CRLF line separators; core.autocrlf unset
```
keys: Ctrl+Enter
```after
dialog "Line Separators Warning": "You are about to commit CRLF line separators to the Git repository." buttons: "Fix and Commit", "Commit As Is", "Cancel"
```

### VCS-CMT-052 case 1: detached HEAD
settings: Warn when committing in detached HEAD or during rebase = on
language: plain
```before
HEAD detached at a1
```
keys: Ctrl+Enter
```after
warning titled "Commit in Detached HEAD" for the repository path; commit proceeds only after confirmation
```

### VCS-CMT-053 case 1: large file
settings: Warn when committing files larger than 50 MB
language: plain
```before
one new file of 60 MB ticked
```
keys: Ctrl+Enter
```after
warning text begins "You are about to commit a large file into the repository: 1 file with the total size of 60MB."
```

### VCS-CMT-056 case 1: success notification
settings: default
language: plain
```before
two files committed
```
keys: Ctrl+Enter
```after
notification "2 files committed"
```

### VCS-CMT-059 case 1: commit and push preview
settings: Show Push dialog for Commit and Push = on
language: plain
```before
branch main tracks origin/main; one file ticked, message "x"
```
keys: Ctrl+Alt+K
```after
commit created; then dialog titled "Push Commits to origin/main" listing the new commit "x"
```

### VCS-CMT-070 case 1: staging area groups
settings: Enable staging area on
language: plain
```before
work: M A.java (unstaged), M B.java (staged), ? C.java
```
keys: (Commit window)
```after
tree top to bottom: "Staged" (B.java), "Unstaged" (A.java), "Untracked" (C.java); no changelist toolbar group
```

### VCS-CMT-070 case 2: stage with Ctrl+Alt+A
settings: staging area on
language: plain
```before
A.java unstaged selected
```
keys: Ctrl+Alt+A
```after
index: A.java staged; A.java appears under "Staged"
```

### VCS-CMT-070 case 3: empty state
settings: staging area on
language: plain
```before
clean repo
```
keys: (view)
```after
status text "Nothing to commit, working tree is clean" with link "Getting started with staging area"
```

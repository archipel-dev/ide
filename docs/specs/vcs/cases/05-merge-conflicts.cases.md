# Cases for 05 (merge conflicts)

Notation as in `01-changes-and-changelists.cases.md`. Merge-viewer cases give the three inputs as
`left` (yours), `base`, `right` (theirs) and the expected `result`.

### VCS-MERGE-001 case 1: conflict node
settings: default
language: plain
```before
merge of feature into main stopped; A.java both modified
```
keys: (Local Changes)
```after
tree has node "Merge Conflicts" with link "Resolve" and child A.java; status text "both modified"
```

### VCS-MERGE-002 case 1: banner
settings: default
language: plain
```before
update stopped on conflicts
```
keys: (Local Changes)
```after
banner "Cannot proceed due to merge conflicts" with button "Resolve..."
```

### VCS-MERGE-010 case 1: conflicts dialog
settings: default
language: plain
```before
merge of branch feature into main; conflicted files: src/A.java, src/B.java
```
keys: action Git.ResolveConflicts
```after
dialog "Conflicts"; header "Merging branch feature into branch main"; text "The following files have conflicts:"; table columns Name | Yours (main) | Theirs (feature); rows A.java, B.java; checkbox "Group by directory" (unchecked); buttons "Accept Yours", "Accept Theirs", "Merge...", "Close"
```

### VCS-MERGE-011 case 1: accept theirs
settings: default
language: plain
```before
conflict in A.java: ours "x = 1", theirs "x = 2"; row selected
```
keys: click "Accept Theirs"
```after
A.java content "x = 2"; file staged; row removed; after the last row the dialog closes
```

### VCS-MERGE-011 case 2: accept yours in a rebase
settings: default
language: plain
```before
rebase of feature onto main, conflict in A.java; git ours = main version "x = 1", git theirs = feature commit "x = 2"
```
keys: click "Accept Yours"
```after
A.java content "x = 1" (the main version: in a rebase "Yours" is the branch being rebased onto)
```

### VCS-MERGE-014 case 1: continue rebase prompt
settings: default
language: plain
```before
rebase stopped; last conflicted file resolved
```
keys: (dialog closes)
```after
dialog titled "Resolve Conflicts": "All conflicts have been resolved. Do you want to continue rebase?" with button "Continue Rebase"
```

### VCS-MERGE-020 case 1: accept from left
settings: default
language: plain
```before
left:   "a\nL\nc"
base:   "a\nb\nc"
right:  "a\nR\nc"
```
keys: click the left ">>" arrow on the conflict
```after
result: "a\nL\nc"; the chunk is marked resolved (dimmed); status text "All conflicts resolved"
```

### VCS-MERGE-021 case 1: apply non-conflicting
settings: default
language: plain
```before
left:  "A1\nb\nc\nd\ne"
base:  "a\nb\nc\nd\ne"
right: "a\nb\nc\nd\nE5"
(two non-conflicting changes, no conflict)
```
keys: toolbar "Apply non-conflicting changes:" > "All"
```after
result: "A1\nb\nc\nd\nE5"; status "All conflicts resolved"
```

### VCS-MERGE-022 case 1: status line
settings: default
language: plain
```before
two changes and one conflict unresolved
```
keys: (viewer open)
```after
status text "2 changes, 1 conflict"
after resolving everything: green "All conflicts resolved"
```

### VCS-MERGE-023 case 1: apply with leftovers
settings: default
language: plain
```before
one conflict unresolved
```
keys: click "Apply"
```after
dialog titled "Apply Changes": "There is one conflict left unprocessed." line break "Save changes and mark the conflict resolved anyway?"; buttons "Apply Changes and Mark Resolved", "Continue Merge", "Cancel Merge"
```

### VCS-MERGE-024 case 1: auto-apply setting
settings: Automatically apply non-conflicting changes = on
language: plain
```before
left/base/right as in VCS-MERGE-021 case 1 plus one real conflict on line 3
```
keys: open merge viewer
```after
result already contains both non-conflicting changes; only the conflict remains; status "No changes, 1 conflict"
```

### VCS-MERGE-025 case 1: undo accept
settings: default
language: plain
```before
conflict accepted from left (see VCS-MERGE-020 case 1)
```
keys: Ctrl+Z
```after
result returns to base text for the chunk; conflict is unresolved again; undo entry was named "Accept change"
```

### VCS-MERGE-026 case 1: outdated conflict
settings: default
language: plain
```before
merge viewer open; the file is changed on disk by git outside
```
keys: (refresh)
```after
prompt "Merge conflict is outdated. Restore file content prior to conflict resolve start?"
```

### VCS-MERGE-031 case 1: identical edits resolve
settings: default
language: plain
```before
left:  "x y z"
base:  "x q z"
right: "x y z"
```
keys: action Diff.MagicResolveConflicts
```after
result: "x y z"
```

### VCS-MERGE-031 case 2: disjoint word edits on one line
settings: default
language: plain
```before
left:  "A b c"
base:  "a b c"
right: "a b C"
```
keys: action Diff.MagicResolveConflicts
```after
result: "A b C"
```

### VCS-MERGE-031 case 3: true conflict remains
settings: default
language: plain
```before
left:  "L"
base:  "b"
right: "R"
```
keys: action Diff.MagicResolveConflicts
```after
unchanged: conflict remains; result still shows the base text "b" for the chunk
```

### VCS-MERGE-013 case 1: resolve all simple
settings: iterative resolution on (default)
language: plain
```before
three conflicted files: F1 (simple), F2 (simple), F3 (true conflict)
```
keys: click "Resolve All Simple Conflicts"
```after
status text "2 conflicts resolved. 1 conflict in 1 file still requires attention"
F1 and F2 rows show resolved counts; F3 remains
```

### VCS-MERGE-013 case 2: accept over merged changes
settings: default
language: plain
```before
file F3 contains merged changes; select F3, click "Accept 'Theirs'"
```
keys: (confirm)
```after
dialog "Overwrite Changes": "This file already contains merged changes. Accepting changes from 'Theirs' will discard and overwrite them." buttons "Discard and Accept", cancel
```

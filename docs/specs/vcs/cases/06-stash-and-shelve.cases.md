# Cases for 06 (stash, shelf, patches)

Notation as in `01-changes-and-changelists.cases.md`.

### VCS-STASH-001 case 1: stash dialog
settings: default
language: plain
```before
work: M A.java, ? n.txt
```
keys: action Git.Stash
```after
dialog titled "Stash": "Message:" field, checkbox "Keep index" (unchecked), checkbox "Include untracked" (unchecked), button "Create Stash"
```

### VCS-STASH-001 case 2: create stash
settings: default
language: plain
```before
work: M A.java; dialog: Message "wip a", Include untracked off
```
keys: click "Create Stash"
```after
work: clean except n.txt untouched; stash list: stash@{0} "On main: wip a" containing A.java
```

### VCS-STASH-002 case 1: stash silently
settings: staging area on
language: plain
```before
work: M A.java
```
keys: action Git.Stash.Silently
```after
work clean; notification "Files stashed successfully" with link "View Git stashes"
```

### VCS-STASH-003 case 1: unstash dialog
settings: default
language: plain
```before
stashes: stash@{0} "wip a", stash@{1} "wip b"; branch main
```
keys: action Git.Unstash
```after
dialog titled "Unstash Changes" ("Unstash Changes in <root>" when several roots): "Current branch:" main; "Stashes:" list [stash@{0} wip a, stash@{1} wip b]; buttons View, Drop, Clear; checkboxes "Pop stash" (unchecked), "Reinstate index" (unchecked); field "As new branch:"; action button "Apply Stash"
checking "Pop stash" changes the button to "Pop Stash"; typing a branch name changes it to "Branch"
```

### VCS-STASH-003 case 2: clear confirmation
settings: default
language: plain
```before
Unstash dialog, stashes exist
```
keys: click "Clear"
```after
confirmation "Remove All Stashes?" with text "Remove all stashes? This cannot be undone."
```

### VCS-STASH-003 case 3: drop confirmation
settings: default
language: plain
```before
stash@{0} "wip a" selected
```
keys: click "Drop"
```after
confirmation titled "Remove Stash stash@{0}?" text: "Do you want to remove stash@{0}?" then the message in quotes: "wip a"
```

### VCS-STASH-004 case 1: stash tab context menu
settings: default (separate tab)
language: plain
```before
Stash tab with entry selected
```
keys: right click
```after
menu: Pop, Apply, Unstash..., Drop, Clear
tree root node text "Stashes"
```

### VCS-STASH-004 case 2: empty state
settings: default
language: plain
```before
no stashes, no shelves
```
keys: open the Stashes and Shelves tab
```after
empty text "No stashes or shelves added." with link "Getting started with Stash and Shelf"
```

### VCS-SHELF-001 case 1: shelve silently
settings: default
language: plain
```before
lists: Changes*: A.java (M)
```
keys: Ctrl+Shift+H
```after
A.java reverts to HEAD content; Shelf tab shows one changelist named "Changes" (or auto name) containing A.java; notification "Changes shelved successfully"
```

### VCS-SHELF-002 case 1: shelf context menu
settings: default
language: plain
```before
Shelf tab with one shelved changelist selected
```
keys: right click
```after
menu: Unshelve... (Ctrl+Shift+U), Restore Applied Shelved Change, Show Diff, Show Diff in a New Tab or Window, Compare with Local, Create Patch..., Copy as Patch to Clipboard, Import Patches..., separator, Rename... (F2), Delete
```

### VCS-SHELF-003 case 1: unshelve silently
settings: default
language: plain
```before
shelf has "Changes" with A.java; work tree has A.java at HEAD
```
keys: Ctrl+Alt+U
```after
A.java is modified again in the original changelist; the shelf entry is marked already unshelved
```

### VCS-SHELF-004 case 1: delete and undo
settings: default
language: plain
```before
shelf entry "Fix" with one file
```
keys: Delete, confirm, then click "Undo Shelf Deletion"
```after
after delete: notification text "one file and the 'Fix' changelist deleted from shelf successfully"; entry appears under "Recently Deleted"
after undo: entry back in the main list
```

### VCS-PATCH-001 case 1: create patch
settings: default
language: plain
```before
HEAD A.java: "a\nb\nc\nd\ne\nf\ng"; work: line d -> D
```
keys: Create Patch from Local Changes..., to file p.patch
```after
p.patch contains one hunk "@@ -1,7 +1,7 @@" with 3 context lines above and below the "-d"/"+D" lines (a file header "--- a/A.java" / "+++ b/A.java")
```

### VCS-PATCH-001 case 2: base path error
settings: default
language: plain
```before
dialog with base path empty
```
keys: click Create Patch
```after
error text "Base path cannot be empty!"
```

### VCS-PATCH-002 case 1: apply clean patch
settings: default
language: plain
```before
patch from case above; work tree A.java at HEAD
```
keys: Apply Patch... choose p.patch, OK
```after
A.java has line d changed to D; notification "Patch successfully applied"; the change is in the active changelist
```

### VCS-PATCH-002 case 2: already applied
settings: default
language: plain
```before
A.java already contains D
```
keys: Apply the same patch
```after
message "All changes in the specified patch are already contained in the code"
```

### VCS-PATCH-004 case 1: rollback dialog
settings: default
language: plain
```before
work: M A.java, A new.txt
```
keys: Ctrl+Alt+Z
```after
dialog "Rollback Changes" listing A.java and new.txt; checkbox "Delete local copies of added files" (unchecked); after OK: A.java at HEAD, new.txt unversioned (still on disk)
```

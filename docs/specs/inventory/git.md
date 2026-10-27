# Git

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (129)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `ChangesView.GroupBy.Repository` | Repository | `ChangesView.GroupBy` |  | MISSING | `platform/dvcs-impl` |
| `CheckIgnoredAndNotExcludedDirectories` | Check Ignored but not Excluded Directories |  |  | MISSING | `platform/dvcs-impl` |
| `CopyPathFromRepositoryRootProvider` | Path From Repository Root | `CopyFileReference` |  | MISSING | `plugins/git4idea` |
| `Git.Add` | Add | `Git.FileActions` | as `ChangesView.AddUnversioned` | MISSING | `plugins/git4idea` |
| `Git.AddCommitToRemoteBranch` | Add Commits to Remote Branch… | `Git.Log.ContextMenu` |  | MISSING | `plugins/git4idea` |
| `Git.Branches` | Branches… | `Git.MainMenu`, `GitRepositoryActions` | `control shift BACK_QUOTE` | MISSING | `plugins/git4idea` |
| `Git.Branches.Popup.Fetch` | (set in code) |  |  | MISSING | `plugins/git4idea` |
| `Git.BrowseRepoAtRevision` | Show Repository at Revision | `Git.Log.ContextMenu.CheckoutBrowse` |  | MISSING | `plugins/git4idea` |
| `Git.ChangesView.AcceptTheirs` | (set in code) | `Git.ChangesView.Conflicts` |  | MISSING | `plugins/git4idea` |
| `Git.ChangesView.AcceptYours` | (set in code) | `Git.ChangesView.Conflicts` |  | MISSING | `plugins/git4idea` |
| `Git.ChangesView.Merge` | (set in code) | `Git.ChangesView.Conflicts` |  | MISSING | `plugins/git4idea` |
| `Git.Checkout.Update` | Checkout and Update | `Git.Branch.Checkout` |  | MISSING | `plugins/git4idea` |
| `Git.CheckoutRevision` | Checkout Revision |  |  | MISSING | `plugins/git4idea` |
| `Git.CherryPick.Abort` | Abort Cherry-Pick | `Git.MainMenu`, `GitRepositoryActions`, `Git.Ongoing.Rebase.Actions` (+1) |  | MISSING | `plugins/git4idea` |
| `Git.CherryPick.Continue` | Continue Cherry-Pick | `Git.MainMenu`, `GitRepositoryActions`, `Git.Ongoing.Rebase.Actions` (+1) |  | MISSING | `plugins/git4idea` |
| `Git.Cleanup.Branches` | Clean Up Branches… | `Vcs.Internal` |  | MISSING | `plugins/git4idea` |
| `Git.Clone` | Clone… | `Git.MainMenu`, `GitRepositoryActions` |  | MISSING | `plugins/git4idea` |
| `Git.Commit.And.Push.Executor` | (set in code) | `Vcs.Commit.PrimaryCommitActions` | `control alt K` | MISSING | `plugins/git4idea` |
| `Git.Commit.Stage` | Commit… | `Git.MainMenu`, `VcsToolbarActions`, `VcsNavBarToolbarActions` (+1) | as `CheckinProject` | MISSING | `plugins/git4idea` |
| `Git.CompareWithBranch` | Compare with Branch or Tag… | `Git.FileActions` |  | MISSING | `plugins/git4idea` |
| `Git.Configure.Remotes` | Manage Remotes… | `Git.MainMenu`, `GitRepositoryActions` |  | MISSING | `plugins/git4idea` |
| `Git.CreateNewBranch` | New Branch… | `Git.MainMenu`, `GitRepositoryActions`, `Git.Branches.List` | `control alt N` | MISSING | `plugins/git4idea` |
| `Git.CreateNewBranch.FromCommit` | New Branch… | `Git.Log.ContextMenu` | as `Git.CreateNewBranch` | MISSING | `plugins/git4idea` |
| `Git.CreateNewTag` | New Tag… | `Git.FileHistory.ContextMenu`, `Git.Log.ContextMenu` |  | MISSING | `plugins/git4idea` |
| `Git.CreateNewWorkingTree` | New Worktree… | `Git.MainMenu`, `Git.Branches.List`, `Git.Branch.Backend` (+1) |  | MISSING | `plugins/git4idea` |
| `Git.Drop.Commits` | Drop Commits | `Git.Log.ContextMenu` |  | MISSING | `plugins/git4idea` |
| `Git.Drop.Selected.Changes` | Drop Selected Changes | `Vcs.RepositoryChangesBrowserMenu` |  | MISSING | `plugins/git4idea` |
| `Git.Fetch` | Fetch | `Git.MainMenu`, `GitRepositoryActions` |  | MISSING | `plugins/git4idea` |
| `Git.FindMergedLocalBranches` | Find Merged Local Branches |  |  | MISSING | `plugins/git4idea` |
| `Git.Fixup.To.Commit` | Fixup… | `Git.FileHistory.ContextMenu`, `Git.Log.ContextMenu` |  | MISSING | `plugins/git4idea` |
| `Git.InMemory.Extract.Selected.Changes` | Extract Selected Changes to Separate Commit… | `Vcs.RepositoryChangesBrowserMenu` |  | MISSING | `plugins/git4idea` |
| `Git.Init` | Create Git Repository… | `Vcs.Import`, `Vcs.Operations.Popup.NonVcsAware`, `Vcs.ToolWindow.CreateRepository` (+1) |  | MISSING | `plugins/git4idea` |
| `Git.Interactive.Rebase` | Interactively Rebase from Here… | `Git.Log.ContextMenu` |  | MISSING | `plugins/git4idea` |
| `Git.Log` | Open External Git Repository Log… | `Vcs.Browse`, `Vcs.Log.ToolWindow.TabActions.DropDown` |  | MISSING | `plugins/git4idea` |
| `Git.Log.Branches.Change.Branch.Filter` | Update Branch Filter in Log |  | `mouse button1 doubleClick`, `ENTER` | MISSING | `plugins/git4idea` |
| `Git.Log.Branches.Change.Branch.Filter.On.Selection` | Update Branch Filter | `Git.Log.Branches.Settings` |  | MISSING | `plugins/git4idea` |
| `Git.Log.Branches.Navigate.Log.To.Branch.On.Selection` | Navigate Log to Branch Head | `Git.Log.Branches.Settings` |  | MISSING | `plugins/git4idea` |
| `Git.Log.Branches.Navigate.Log.To.Selected.Branch` | Navigate Log to Selected Branch Head |  | as `SelectIn` | MISSING | `plugins/git4idea` |
| `Git.Log.Hide.Branches` | Hide Git Branches |  |  | MISSING | `plugins/git4idea` |
| `Git.Log.HighlightCherryPickedCommits` | Not Cherry-Picked Commits | `Git.Log.Toolbar` |  | MISSING | `plugins/git4idea` |
| `Git.Log.Show.Command` | Show Git Log for Command | `Git.Log.Internal` |  | MISSING | `plugins/git4idea` |
| `Git.Merge` | Merge… | `Git.MainMenu`, `GitRepositoryActions` |  | MISSING | `plugins/git4idea` |
| `Git.Merge.Abort` | Abort Merge | `Git.MainMenu.MergeActions`, `Git.Ongoing.Rebase.Actions`, `GitMergeRebaseWidgetGroup` |  | MISSING | `plugins/git4idea` |
| `Git.Merge.Commit` | Commit Merge | `Git.MainMenu.MergeActions`, `GitMergeRebaseWidgetGroup` |  | MISSING | `plugins/git4idea` |
| `Git.New.Branch.In.Log` | (set in code) |  | `control alt N` | MISSING | `plugins/git4idea` |
| `Git.OpenExcludeFile` | (set in code) |  |  | MISSING | `plugins/git4idea` |
| `Git.PrefillRecentPushTargets` | Prefill Recent Push Targets |  |  | MISSING | `plugins/git4idea` |
| `Git.Pull` | Pull… | `Git.MainMenu`, `GitRepositoryActions` |  | MISSING | `plugins/git4idea` |
| `Git.PushSelectedCommits` | Push Selected Commits… | `Git.Log.ContextMenu` |  | MISSING | `plugins/git4idea` |
| `Git.PushUpToCommit` | Push All up to Here… | `Git.Log.ContextMenu` |  | MISSING | `plugins/git4idea` |
| `Git.Rebase` | Rebase… | `Git.MainMenu`, `GitRepositoryActions` |  | MISSING | `plugins/git4idea` |
| `Git.Rebase.Abort` | Abort Rebase | `Git.MainMenu.RebaseActions`, `Git.Ongoing.Rebase.Actions`, `GitMergeRebaseWidgetGroup` |  | MISSING | `plugins/git4idea` |
| `Git.Rebase.Continue` | Continue Rebase | `Git.MainMenu.RebaseActions`, `Git.Ongoing.Rebase.Actions`, `GitMergeRebaseWidgetGroup` |  | MISSING | `plugins/git4idea` |
| `Git.Rebase.Skip` | Skip Commit | `Git.MainMenu.RebaseActions`, `Git.Ongoing.Rebase.Actions` |  | MISSING | `plugins/git4idea` |
| `Git.Ref.Compare.With` | (set in code) | `Git.Branch.Backend` |  | MISSING | `plugins/git4idea` |
| `Git.Ref.Diff.With.Local` | (set in code) | `Git.Branch.Backend` |  | MISSING | `plugins/git4idea` |
| `Git.Rename.Local.Branch` | (set in code) | `Git.Branch.Backend` | `F2`, `Shift F6` | MISSING | `plugins/git4idea` |
| `Git.Reset` | Reset HEAD… | `Git.MainMenu`, `GitRepositoryActions` |  | MISSING | `plugins/git4idea` |
| `Git.Reset.In.Log` | Reset Current Branch to Here… | `Git.Log.ContextMenu` |  | MISSING | `plugins/git4idea` |
| `Git.ResolveConflicts` | Resolve Conflicts… | `Git.MainMenu`, `Git.ContextMenu`, `GitMergeRebaseWidgetGroup` |  | MISSING | `plugins/git4idea` |
| `Git.Revert.Abort` | Abort Revert | `Git.MainMenu`, `GitRepositoryActions`, `Git.Ongoing.Rebase.Actions` (+1) |  | MISSING | `plugins/git4idea` |
| `Git.Revert.In.Log` | Revert Commit | `Git.FileHistory.ContextMenu`, `Git.Log.ContextMenu` |  | MISSING | `plugins/git4idea` |
| `Git.RevertResolved` | Revert Resolved | `Git.MainMenu`, `ChangesViewPopupMenu`, `Git.ContextMenu` (+1) |  | MISSING | `plugins/git4idea` |
| `Git.Reword.Commit` | Edit Commit Message… | `Git.Log.ContextMenu` | `F2`, `Shift F6` | MISSING | `plugins/git4idea` |
| `Git.Show.Stage` | Show Staging Area |  |  | MISSING | `plugins/git4idea` |
| `Git.Show.Stash` | Show Git Stash | `Vcs.Show.Toolwindow.Tab`, `Git.MainMenu.LocalChanges` |  | MISSING | `plugins/git4idea` |
| `Git.Show.WorkingTrees` | Worktrees… | `Vcs.Log.ToolWindow.TabActions.DropDown`, `Git.MainMenu` |  | MISSING | `plugins/git4idea` |
| `Git.ShowBranches` | Show Branches | `SegmentedVcsActionsBarGroup` | as `Git.Branches` | MISSING | `plugins/git4idea` |
| `Git.Squash.Commits` | Squash Commits… | `Git.Log.ContextMenu` |  | MISSING | `plugins/git4idea` |
| `Git.Squash.Into.Commit` | Squash Into… | `Git.FileHistory.ContextMenu`, `Git.Log.ContextMenu` |  | MISSING | `plugins/git4idea` |
| `Git.Stage.AcceptTheirs` | (set in code) | `Git.Stage.Tree.Menu` |  | MISSING | `plugins/git4idea` |
| `Git.Stage.AcceptYours` | (set in code) | `Git.Stage.Tree.Menu` |  | MISSING | `plugins/git4idea` |
| `Git.Stage.Add` | (set in code) | `Git.Stage.Tree.Menu` | as `ChangesView.AddUnversioned` | MISSING | `plugins/git4idea` |
| `Git.Stage.Add.All` | Stage All |  |  | MISSING | `plugins/git4idea` |
| `Git.Stage.Add.No.Content` | (set in code) | `Git.Stage.Tree.Menu` |  | MISSING | `plugins/git4idea` |
| `Git.Stage.Add.Tracked` | Stage All Tracked | `Git.Stage.Toolbar` |  | MISSING | `plugins/git4idea` |
| `Git.Stage.Compare.Local.Staged` | Compare with Staged Version | `Git.Stage.Local.File.Menu` |  | MISSING | `plugins/git4idea` |
| `Git.Stage.Compare.Staged.Head` | Compare with HEAD Version | `Git.Stage.Index.File.Menu` |  | MISSING | `plugins/git4idea` |
| `Git.Stage.Compare.Staged.Local` | Compare with Local Version | `Git.Stage.Index.File.Menu` |  | MISSING | `plugins/git4idea` |
| `Git.Stage.Compare.Three.Versions` | Compare HEAD, Staged and Local Versions | `Git.Stage.Local.File.Menu`, `Git.Stage.Index.File.Menu` |  | MISSING | `plugins/git4idea` |
| `Git.Stage.Disable` | Changelists | `Git.Stage.Configure` |  | MISSING | `plugins/git4idea` |
| `Git.Stage.Enable` | Staging Area | `Git.Stage.Configure` |  | MISSING | `plugins/git4idea` |
| `Git.Stage.Merge` | (set in code) | `Git.Stage.Tree.Menu` |  | MISSING | `plugins/git4idea` |
| `Git.Stage.Reset` | (set in code) | `Git.Stage.Tree.Menu` | as `ChangesView.Revert` | MISSING | `plugins/git4idea` |
| `Git.Stage.Revert` | (set in code) | `Git.Stage.Tree.Menu` | as `ChangesView.Revert` | MISSING | `plugins/git4idea` |
| `Git.Stage.Show.Local` | Show Local Version | `Git.Stage.Index.File.Menu` |  | MISSING | `plugins/git4idea` |
| `Git.Stage.Show.Staged` | Show Staged Version | `Git.Stage.Local.File.Menu` |  | MISSING | `plugins/git4idea` |
| `Git.Stage.Stash.Files` | (set in code) | `Git.Stage.Tree.Menu` |  | MISSING | `plugins/git4idea` |
| `Git.Stage.ThreeSideDiff` | Compare HEAD, Staged and Local Versions | `Git.Stage.Tree.Menu` |  | MISSING | `plugins/git4idea` |
| `Git.Stage.ToggleCommitAll` | Commit All when Nothing Is Staged | `CommitView.GearActions` |  | MISSING | `plugins/git4idea` |
| `Git.Stage.ToggleIgnored` | (set in code) | `Git.Stage.Ui.Settings` |  | MISSING | `plugins/git4idea` |
| `Git.Stash` | Stash Changes… | `Git.MainMenu.LocalChanges`, `GitRepositoryActions` |  | MISSING | `plugins/git4idea` |
| `Git.Stash.Apply` | Apply | `Git.Stash.Operations.ContextMenu` |  | MISSING | `plugins/git4idea` |
| `Git.Stash.Clear` | Clear | `Git.Stash.Operations.ContextMenu` |  | MISSING | `plugins/git4idea` |
| `Git.Stash.Drop` | Drop | `Git.Stash.Operations.ContextMenu` | as `$Delete` | MISSING | `plugins/git4idea` |
| `Git.Stash.Pop` | Pop | `Git.Stash.Operations.ContextMenu` |  | MISSING | `plugins/git4idea` |
| `Git.Stash.Refresh` | Refresh Stashes |  |  | MISSING | `plugins/git4idea` |
| `Git.Stash.Silently` | Stash Silently | `Git.Stage.Toolbar` |  | MISSING | `plugins/git4idea` |
| `Git.Stash.Toggle.Split.Preview` | Preview Diff | `Vcs.SavedPatches.ChangesBrowser.Toolbar` |  | MISSING | `plugins/git4idea` |
| `Git.Stash.UnstashAs` | Unstash… | `Git.Stash.Operations.ContextMenu` |  | MISSING | `plugins/git4idea` |
| `Git.Tag` | New Tag… | `Git.MainMenu`, `GitRepositoryActions` |  | MISSING | `plugins/git4idea` |
| `Git.Tag.Push` | (set in code) |  |  | MISSING | `plugins/git4idea` |
| `Git.Toolbar.ShowMoreActions` | Version Control | `SegmentedVcsActionsBarGroup` |  | MISSING | `plugins/git4idea` |
| `Git.Uncommit` | Undo Commit… | `Git.Log.ContextMenu` |  | MISSING | `plugins/git4idea` |
| `Git.Unshallow` | Unshallow repository | `Git.MainMenu` |  | MISSING | `plugins/git4idea` |
| `Git.Unstash` | Unstash Changes… | `Git.MainMenu.LocalChanges`, `GitRepositoryActions` |  | MISSING | `plugins/git4idea` |
| `Git.WorkingTrees.Open` | Open Worktree | `Git.WorkingTrees.ToolwindowGroup.Popup` | `mouse button1 doubleClick` | MISSING | `plugins/git4idea` |
| `Git.WorkingTrees.OpenReview` | Open Review | `Git.WorkingTrees.ToolwindowGroup.Popup` |  | MISSING | `plugins/git4idea` |
| `Git.WorkingTrees.Prune` | Prune | `Git.WorkingTrees.ToolwindowGroup.Toolbar` |  | MISSING | `plugins/git4idea` |
| `Git.WorkingTrees.Refresh` | Refresh | `Git.WorkingTrees.ToolwindowGroup.Toolbar` | as `Synchronize` | MISSING | `plugins/git4idea` |
| `Git.WorkingTrees.Remove` | Delete… | `Git.WorkingTrees.ToolwindowGroup.Toolbar`, `Git.WorkingTrees.ToolwindowGroup.Popup` | as `$Delete` | MISSING | `plugins/git4idea` |
| `Git.WorkingTrees.ToggleLock` | Lock | `Git.WorkingTrees.ToolwindowGroup.Popup` |  | MISSING | `plugins/git4idea` |
| `Vcs.CherryPick` | (set in code) | `Dvcs.Log.ContextMenu`, `Dvcs.FileHistory.ContextMenu`, `Dvcs.Log.Toolbar` (+1) |  | MISSING | `platform/dvcs-impl` |
| `Vcs.CopyCurrentBranchName` | Copy Branch Name |  |  | MISSING | `platform/dvcs-impl` |
| `Vcs.Push` | Push… | `VcsToolbarActions`, `VcsNavBarToolbarActions`, `SegmentedVcsActionsBarGroup` (+5) | `control shift K` | MISSING | `platform/dvcs-impl` |
| `Vcs.Push.Force` | Force Push | `Vcs.Push.Actions` |  | MISSING | `platform/dvcs-impl` |
| `Vcs.Push.Simple` | (set in code) | `Vcs.Push.Actions` |  | MISSING | `platform/dvcs-impl` |
| `git.branches.group.by.directory` | Group by Directory | `Git.Branches.Popup.Settings`, `Git.Log.Branches.Settings`, `Git.Compare.With.Branch.Popup.Settings` |  | MISSING | `plugins/git4idea` |
| `git.branches.group.by.repository` | Group by Repository | `Git.Branches.Popup.Settings`, `Git.Log.Branches.Settings` |  | MISSING | `plugins/git4idea` |
| `git.branches.popup.filter.by.action` | Actions | `Git.Branches.Popup.Settings` |  | MISSING | `plugins/git4idea` |
| `git.branches.popup.filter.by.repository` | Repositories | `Git.Branches.Popup.Settings` |  | MISSING | `plugins/git4idea` |
| `git.branches.popup.resize` | (set in code) | `Git.Branches.Popup.Settings`, `Git.Compare.With.Branch.Popup.Settings` |  | MISSING | `plugins/git4idea` |
| `git.branches.show.recent` | Show Recent Branches | `Git.Branches.Popup.Settings`, `Git.Compare.With.Branch.Popup.Settings` |  | MISSING | `plugins/git4idea` |
| `git.branches.show.tags` | Show Tags | `Git.Log.Branches.Settings`, `Git.Compare.With.Branch.Popup.Settings`, `Git.Branches.Popup.Settings` |  | MISSING | `plugins/git4idea` |
| `git.update.force.pushed.branch` | Update Force-Pushed Branch |  |  | MISSING | `plugins/git4idea` |
| `git4idea.commands.TestGitHttpLoginDialogAction` | Test Git Login Dialog |  |  | MISSING | `plugins/git4idea` |
| `gpg.agent.configuration` | Configure GPG Agent |  |  | MISSING | `plugins/git4idea` |
| `main.toolbar.git.Branches` | VCS Widget | `MainToolbarVCSGroup` |  | MISSING | `plugins/git4idea` |
| `main.toolbar.git.MergeRebase` | VCS Merge/Rebase Widget | `MainToolbarVCSGroup` |  | MISSING | `plugins/git4idea` |

## Settings pages (1)

| Id | Display name | Group | Parent | Scope | Spec |
|---|---|---|---|---|---|
| `vcs.Git` | Git |  | project.propVCSSupport.Mappings | project | MISSING |

## Status bar widgets (2)

| Id | Factory | Order | Spec |
|---|---|---|---|
| `light.edit.git` | `LightGitStatusBarWidgetFactory` |  | MISSING |
| `git` | `Factory` | after CodeStyleStatusBarWidget,before ReadOnlyAttribute | MISSING |

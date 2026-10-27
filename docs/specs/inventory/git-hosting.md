# GitHub and GitLab

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (47)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `Code.Review.Editor.New.Comment` | (set in code) | `EditorPopupMenu`, `Code.Review.Diff.Editor.Popup` | `control shift X` | MISSING | `platform/collaboration-tools` |
| `Code.Review.Editor.Show.Diff` | (set in code) | `EditorPopupMenu` |  | MISSING | `platform/collaboration-tools` |
| `CodeReview.NextComment` | Next Comment |  | as `NextOccurence` | MISSING | `platform/collaboration-tools` |
| `CodeReview.PreviousComment` | Previous Comment |  | as `PreviousOccurence` | MISSING | `platform/collaboration-tools` |
| `GitHub.Timeline.DeleteMergedBranch` | Delete Branch |  |  | MISSING | `plugins/github` |
| `GitLab.Clone` | Clone Repository from GitLab… | `GitLab.Main.Group` |  | MISSING | `plugins/gitlab` |
| `GitLab.Create.Snippet` | Create Snippet… | `EditorPopupMenu`, `ProjectViewPopupMenu`, `EditorTabPopupMenu` (+2) |  | MISSING | `plugins/gitlab` |
| `GitLab.Merge.Request.Branch.Update` | Update to Enable Review Mode… | `GitLab.Merge.Request.Review.Editor.Branch.Popup.Actions` |  | MISSING | `plugins/gitlab` |
| `GitLab.Merge.Request.Changes.MarkNotViewed` | (set in code) | `GitLab.Merge.Request.Changes.Popup` |  | MISSING | `plugins/gitlab` |
| `GitLab.Merge.Request.Changes.MarkViewed` | (set in code) | `GitLab.Merge.Request.Changes.Popup` |  | MISSING | `plugins/gitlab` |
| `GitLab.Merge.Request.CheckoutBranch` | Checkout Branch | `GitLab.Merge.Request.List.Actions` |  | MISSING | `plugins/gitlab` |
| `GitLab.Merge.Request.Copy.Link` | Copy Merge Request URL | `GitLab.Merge.Request.List.Actions`, `GitLab.Merge.Request.Details.Popup`, `GitLab.Merge.Request.Timeline.Popup` (+2) |  | MISSING | `plugins/gitlab` |
| `GitLab.Merge.Request.Create` | Create Merge Request… | `Git.Menu`, `GitLab.Main.Group` |  | MISSING | `plugins/gitlab` |
| `GitLab.Merge.Request.List.Refresh` | Refresh List | `GitLab.Merge.Request.List.Actions` | as `Refresh` | MISSING | `plugins/gitlab` |
| `GitLab.Merge.Request.Open.Link` | View Merge Request in Browser | `GitLab.Merge.Request.List.Actions`, `GitLab.Merge.Request.Details.Popup`, `GitLab.Merge.Request.Timeline.Popup` (+2) |  | MISSING | `plugins/gitlab` |
| `GitLab.Merge.Request.OpenInWorktree` | Open in Worktree | `GitLab.Merge.Request.List.Actions` |  | MISSING | `plugins/gitlab` |
| `GitLab.Merge.Request.Refresh` | Refresh Merge Request | `GitLab.Merge.Request.Details.Popup`, `GitLab.Merge.Request.Timeline.Popup`, `GitLab.Merge.Request.Changes.Popup` | as `Refresh` | MISSING | `plugins/gitlab` |
| `GitLab.Merge.Request.Show` | Show Merge Request | `GitLab.Merge.Request.List.Actions` |  | MISSING | `plugins/gitlab` |
| `GitLab.Merge.Request.Show.In.Toolwindow` | Show Merge Request in Tool Window | `GitLab.Merge.Request.Review.Editor.Branch.Popup.Actions` |  | MISSING | `plugins/gitlab` |
| `GitLab.Merge.Request.Show.List` | View Merge Requests | `GitLab.Main.Group` |  | MISSING | `plugins/gitlab` |
| `GitLab.Merge.Request.Timeline.ShowEvents` | Show Merge Request Events | `GitLab.Merge.Request.Timeline.Popup` |  | MISSING | `plugins/gitlab` |
| `GitLab.MergeRequest.Review.Mode.Toggle` | Review Mode | `GitLab.Merge.Request.Review.Editor.Branch.Popup.Actions` |  | MISSING | `plugins/gitlab` |
| `GitLab.MergeRequest.Review.Submit` | (set in code) | `GitLab.Merge.Request.Review.Editor.Branch.Popup.Actions` |  | MISSING | `plugins/gitlab` |
| `GitLab.Open.Settings` | Manage GitLab Accounts… | `GitLab.Main.Group` |  | MISSING | `plugins/gitlab` |
| `GitLab.Share` | Share Project on GitLab | `GitLab.Main.Group`, `Vcs.Import`, `Vcs.ToolbarWidget.ShareProject` |  | MISSING | `plugins/gitlab` |
| `Github.Break.Api.Requests` | Break GitHub API Requests |  |  | MISSING | `plugins/github` |
| `Github.Clone` | Clone Repository from GitHub… | `GitHub.MainMenu` |  | MISSING | `plugins/github` |
| `Github.Create.Gist` | Create Gist… | `EditorPopupMenu`, `ProjectViewPopupMenu`, `EditorTabPopupMenu` (+2) |  | MISSING | `plugins/github` |
| `Github.Create.Pull.Request` | Create Pull Request… | `Git.Menu`, `GitHub.MainMenu` |  | MISSING | `plugins/github` |
| `Github.Open.Settings` | Manage GitHub Accounts… | `GitHub.MainMenu` |  | MISSING | `plugins/github` |
| `Github.Pull.Request.Branch.Update` | Update to Enable Review Mode… | `Github.Pull.Request.Branch.Popup.Actions` |  | MISSING | `plugins/github` |
| `Github.Pull.Request.Review.In.Editor.Toggle` | Review Mode | `Github.Pull.Request.Branch.Popup.Actions` |  | MISSING | `plugins/github` |
| `Github.Pull.Request.Show.In.Toolwindow` | Show Pull Request in the Tool Window | `Github.Pull.Request.Branch.Popup.Actions` |  | MISSING | `plugins/github` |
| `Github.PullRequest.Changed.MarkViewed.Toggle` | Toggle viewed/not viewed state | `Github.PullRequest.Changes.Popup`, `EditorPopupMenu`, `Code.Review.Diff.Editor.Popup` | `control shift S` | MISSING | `plugins/github` |
| `Github.PullRequest.CheckoutBranch` | Checkout Branch | `Github.PullRequest.ToolWindow.List.Popup` |  | MISSING | `plugins/github` |
| `Github.PullRequest.Copy.Link` | Copy Pull Request URL | `Github.PullRequest.ToolWindow.List.Popup`, `Github.PullRequest.Details.Popup`, `Github.PullRequest.Changes.Popup` (+1) |  | MISSING | `plugins/github` |
| `Github.PullRequest.Details.Reload` | (set in code) | `Github.PullRequest.Details.Popup`, `Github.PullRequest.Changes.Popup` | as `Refresh` | MISSING | `plugins/github` |
| `Github.PullRequest.List.Reload` | Refresh List | `Github.PullRequest.ToolWindow.List.Popup` | as `Refresh` | MISSING | `plugins/github` |
| `Github.PullRequest.Open.Link` | View Pull Request in Browser | `Github.PullRequest.ToolWindow.List.Popup`, `Github.PullRequest.Details.Popup`, `Github.PullRequest.Changes.Popup` (+1) |  | MISSING | `plugins/github` |
| `Github.PullRequest.OpenInWorktree` | Open in Worktree | `Github.PullRequest.ToolWindow.List.Popup` |  | MISSING | `plugins/github` |
| `Github.PullRequest.Review.Submit` | (set in code) | `Github.Pull.Request.Branch.Popup.Actions` |  | MISSING | `plugins/github` |
| `Github.PullRequest.Show` | (set in code) | `Github.PullRequest.ToolWindow.List.Popup` |  | MISSING | `plugins/github` |
| `Github.PullRequest.Timeline.Show` | (set in code) |  |  | MISSING | `plugins/github` |
| `Github.PullRequest.Timeline.Update` | (set in code) | `Github.PullRequest.Timeline.Popup` | as `Refresh` | MISSING | `plugins/github` |
| `Github.Share` | Share Project on GitHub | `GitHub.MainMenu`, `Vcs.Import`, `Vcs.ToolbarWidget.ShareProject` |  | MISSING | `plugins/github` |
| `Github.Sync.Fork` | Sync Fork | `Git.Menu`, `GitHub.MainMenu` |  | MISSING | `plugins/github` |
| `Github.View.Pull.Request` | View Pull Requests | `Git.Menu`, `GitHub.MainMenu` |  | MISSING | `plugins/github` |

## Settings pages (2)

| Id | Display name | Group | Parent | Scope | Spec |
|---|---|---|---|---|---|
| `org.jetbrains.plugins.github.ui.GithubSettingsConfigurable` | GitHub |  | project.propVCSSupport.Mappings | project | MISSING |
| `org.jetbrains.plugins.gitlab.GitLabSettingsConfigurable` | GitLab |  | project.propVCSSupport.Mappings | project | MISSING |

## Tool windows (2)

| Id | Title | Anchor | Flags | Factory | Spec |
|---|---|---|---|---|---|
| `Pull Requests` | Pull Requests | left |  | GHPRToolWindowFactory | MISSING |
| `Merge Requests` | Merge Requests | left |  | GitLabToolWindowFactory | MISSING |

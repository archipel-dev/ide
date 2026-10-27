# 11. Git hosting integrations (GitHub, GitLab, code review)

Status: inventory coverage only. Requirement prefix `VCS-HOST`. Every requirement here is `LATER`:
the integrations need account storage, OAuth/token flows and network access, which the kit and the
current product direction exclude (kit rule: no network or app policy in the kit; see
`docs/specs/README.md`). The rows keep the inventory items traceable and record the visible texts so
that a later plugin can implement them without reading the source.

ic: `plugins/github/**`, `plugins/gitlab/**`, `platform/collaboration-tools/**`,
`plugins/git4idea/hosting/**`, `plugins/github/github-core/resources/messages/GithubBundle.properties`,
`plugins/gitlab/gitlab-core/resources/messages/GitLabBundle.properties`.

Reason for every row: network, account and token management are application services; no
Archipel host surface exists yet. Done-when criterion for the whole file: the Pull Requests and
Merge Requests tool windows list requests of a mocked server and the listed actions work against it.

## 1. Accounts, sharing, cloning

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| VCS-HOST-001 | LATER | `Manage GitHub Accounts...` (`Github.Open.Settings`) and `Manage GitLab Accounts...` (`GitLab.Open.Settings`) open the settings pages `GitHub` (`org.jetbrains.plugins.github.ui.GithubSettingsConfigurable`) and `GitLab` (`org.jetbrains.plugins.gitlab.GitLabSettingsConfigurable`), parent page Version Control. | Both pages exist under Settings > Version Control. |
| VCS-HOST-002 | LATER | `Share Project on GitHub` (`Github.Share`) and `Share Project on GitLab` (`GitLab.Share`) create a remote repository, add it as `origin`, make the initial commit and push; entries in the VCS menu (`Vcs.Import`) and in the VCS widget group `Share Project On`. Failure texts include `on <host>, but initial commit failed:`. | With a mocked server the repository appears and `origin` is set. |
| VCS-HOST-003 | LATER | `Clone Repository from GitHub...` (`Github.Clone`) and `Clone Repository from GitLab...` (`GitLab.Clone`) list the account's repositories in the clone dialog (see VCS-SETUP-001). | Repository list loads from the mock. |
| VCS-HOST-004 | LATER | `Create Gist...` (`Github.Create.Gist`) and `Create Snippet...` (`GitLab.Create.Snippet`) upload the selection or file from the editor/Project view/tab popup. | Mock receives the file content. |
| VCS-HOST-005 | LATER | `Break GitHub API Requests` (`Github.Break.Api.Requests`) is a diagnostic action. | Internal only. |

## 2. Pull requests and merge requests

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| VCS-HOST-010 | LATER | Tool windows `Pull Requests` (GitHub) and `Merge Requests` (GitLab), anchor left: list with search and filters, details with timeline, changes tree, actions shown in popups `Github.PullRequest.ToolWindow.List.Popup` etc. | Both stripes exist when an account is configured. |
| VCS-HOST-011 | LATER | List and open actions: `View Pull Requests` (`Github.View.Pull.Request`), `Show Pull Request in the Tool Window` (`Github.Pull.Request.Show.In.Toolwindow`), `Github.PullRequest.Show`, `Github.PullRequest.Timeline.Show`, `Refresh List` (`Github.PullRequest.List.Reload`, shortcut of `Refresh`), `Github.PullRequest.Details.Reload`, `Github.PullRequest.Timeline.Update`; GitLab: `View Merge Requests` (`GitLab.Merge.Request.Show.List`), `Show Merge Request` (`GitLab.Merge.Request.Show`), `Show Merge Request in Tool Window` (`GitLab.Merge.Request.Show.In.Toolwindow`), `Refresh List` (`GitLab.Merge.Request.List.Refresh`), `Refresh Merge Request` (`GitLab.Merge.Request.Refresh`), `Show Merge Request Events` (`GitLab.Merge.Request.Timeline.ShowEvents`). | Actions registered and enabled when a PR/MR is selected. |
| VCS-HOST-012 | LATER | Create and branch actions: `Create Pull Request...` (`Github.Create.Pull.Request`) and `Create Merge Request...` (`GitLab.Merge.Request.Create`) with the branch-direction panel (`Branch:`, `Base repository:`, `Head repository:`, `Select...`, `Save`; warnings `There is no remote branch for '<b>'. New remote branch will be created on '<r>'.` and `Local branch is not in sync with remote. '<b>' branch will be synced with '<r>'.`); `Checkout Branch` (`Github.PullRequest.CheckoutBranch`, `GitLab.Merge.Request.CheckoutBranch`), `Open in Worktree` (`Github.PullRequest.OpenInWorktree`, `GitLab.Merge.Request.OpenInWorktree`), `Sync Fork` (`Github.Sync.Fork`), `Delete Branch` (`GitHub.Timeline.DeleteMergedBranch`). | Mock PR creation returns a URL link in the notification. |
| VCS-HOST-013 | LATER | Link actions: `Copy Pull Request URL` (`Github.PullRequest.Copy.Link`), `View Pull Request in Browser` (`Github.PullRequest.Open.Link`), `Copy Merge Request URL` (`GitLab.Merge.Request.Copy.Link`), `View Merge Request in Browser` (`GitLab.Merge.Request.Open.Link`). | Copies the canonical URL. |
| VCS-HOST-014 | LATER | Review mode: `Review Mode` toggles (`Github.Pull.Request.Review.In.Editor.Toggle`, `GitLab.MergeRequest.Review.Mode.Toggle`), `Update to Enable Review Mode...` (`Github.Pull.Request.Branch.Update`, `GitLab.Merge.Request.Branch.Update`), `Submit review` (`Github.PullRequest.Review.Submit`, `GitLab.MergeRequest.Review.Submit`), viewed state (`Toggle viewed/not viewed state` `Github.PullRequest.Changed.MarkViewed.Toggle`, `Ctrl+Shift+S`; `GitLab.Merge.Request.Changes.MarkViewed`, `GitLab.Merge.Request.Changes.MarkNotViewed`). Editor comments: `New Comment` (`Code.Review.Editor.New.Comment`, `Ctrl+Shift+X`), `Code.Review.Editor.Show.Diff`, `Next Comment` (`CodeReview.NextComment`, shortcut of `NextOccurence`), `Previous Comment` (`CodeReview.PreviousComment`, shortcut of `PreviousOccurence`). | Registered with the stated shortcuts. |

## Open questions

- OQ-1: whether the first release ships any hosting integration; if yes, an extension surface
  (accounts store, HTTP client with proxy) must be specified before these rows move up.

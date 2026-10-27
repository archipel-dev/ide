# 10. Clone, enabling version control, mappings, ignore files

Status: specification. Requirement prefix `VCS-SETUP`. Executable cases:
`cases/10-clone-and-vcs-setup.cases.md`. Network-dependent parts (cloning, credentials) describe the
contract only: the kit has no network policy (see `docs/specs/README.md`), so these requirements
define the dialogs, validation and results the application layer must provide.

ic: `platform/dvcs-impl/**` (`dvcs/ui/Clone*`, `dvcs/repo`, `dvcs/ignore`),
`platform/dvcs-impl/shared/resources/messages/DvcsBundle.properties`,
`platform/vcs-impl/src/com/intellij/openapi/vcs/{impl,roots,configurable}/**`,
`platform/vcs-impl/src/com/intellij/openapi/vcs/changes/{VcsIgnoreManagerImpl,IgnoredSettingsPanel,
ui/IgnoredSettingsPanel}.kt`, `platform/vcs-impl/src/com/intellij/ide/StartUseVcsAction.java`,
`plugins/git4idea/backend/src/{actions/GitCloneAction,actions/GitInit*,ignore/**,init/**,
checkin,repo}/**`, `plugins/git4idea/backend/resources/intellij.vcs.git.backend.xml`,
`plugins/git4idea/shared/resources/messages/GitBundle.properties`,
`platform/vcs-api/vcs-api-core/resources/messages/VcsBundle.properties`.

## 1. Get from version control (clone)

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| VCS-SETUP-001 | MUST | `Get from Version Control...` (`Vcs.VcsClone`; menu Git > Clone... `Git.Clone` text `Clone...`; Welcome screen and project widget variant `Clone Repository...`; new-project group) opens the clone dialog titled `Clone Repository` (sidebar item `Repository URL`, others provided by plugins/hosting) with `URL:`, `Directory:`/`Clone into:` (destination parent directory), and buttons `Clone` and `Cancel`; with Git a `Version control:` combo (`Git`) and `Test` button (`Testing <url>`, `Connection successful`, `Repository test failed<br/>{0}`). Placeholder text before selection `Click "<name>" to continue`; accessible name `Repository Location`. | Dialog fields appear in this order; `Clone` is disabled until both URL and directory are valid. |
| VCS-SETUP-002 | MUST | Validation (inline, below the field): `Empty repository URL`, `Repository URL is a malformed URL or non-existent directory`, `Repository URL is not a directory`, destination: `Invalid directory path`, `Destination is not a directory`, `The directory already exists and it is not empty`, `This directory is reserved for IDE use and cannot be used for cloning`, `Could not create destination directory<br/>Check your access rights`; the default destination parent is the IDE's projects directory (`~/IdeaProjects`-style) and the target folder is the repository name derived from the URL (last path segment without `.git`). A browse dialog titled `Destination Directory` with description `Select a parent directory for the clone`. | URL `https://h/x/y.git` yields target folder `y`. |
| VCS-SETUP-003 | MUST | Git clone runs `git clone` with progress `Cloning repository...` (`Cloning source repository <url>`); advanced setting `Recursively clone submodules in the project` (`git.clone.recurse.submodules`, default **on**, adds `--recurse-submodules`); optional `Shallow clone with a history truncated to <n> commits` (git 2.x check `Checking Git version...`); failures: `Clone failed` / `Failed to clone repository <url>` with details, `Clone Failed. Destination doesn't exist`, `Unable to find destination directory`; cancel: `Clone canceled`, stop confirmation `Are you sure to you want to stop cloning the <repo> repository?` (title `Stop Cloning`, action `Stop Cloning Repository <name>`). Tooltip on the button `The project will open after cloning is finished`. | Clone a local path repository: the directory appears with a `.git` folder. |
| VCS-SETUP-004 | MUST | After a successful clone the IDE opens the project (asks for window: this/new, trust prompt per platform trust rules), registers the VCS root, and reports `Cloned`-style status in the welcome flow. Credential prompts use `Log In to <url>` dialogs (see VCS-BR-078). | Clone then open: Git branch widget shows the default branch. |
| VCS-SETUP-005 | SHOULD | Hosting clone pages (`Clone Repository from GitHub...` `Github.Clone`, `Clone Repository from GitLab...` `GitLab.Clone`) belong to `11-git-hosting.md`. | See that file. |

## 2. Enable version control integration

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| VCS-SETUP-010 | MUST | For a project without VCS the actions `Enable Version Control Integration...` (`Start.Use.Vcs`; toolbar widget text `Other VCS...`) and `Create Git Repository...` (`Git.Init`; widget text `Create Git Repository...`) are offered in the VCS menu, the VCS widget (`Version Control` popup with `Share Project On` group, `Create Git Repository...`, `Enable Version Control Integration...`) and the Commit/Version Control tool window empty state (`To track changes to code: Share project on <hosting> or Create Git repository... — Version control integration`). | Plain project: those entries are present. |
| VCS-SETUP-011 | MUST | `Enable Version Control Integration...` opens a dialog `Enable Version Control Integration` with `Select a version control system to associate with <project name>:` and a combo of available VCS names (default `Git`), a hint `Version control settings can be configured in <Settings > Version Control>`; OK maps the project root (`<Project>`) to the chosen VCS. | Choose Git and OK: mapping `<Project>` -> Git appears in the settings table. |
| VCS-SETUP-012 | MUST | `Create Git Repository...` opens a directory chooser titled `Create Git Repository` with description `Select directory where the new Git repository will be created.` (defaults to the project base directory); if the directory is already under Git: warning `The selected directory <dir> is already under Git. Are you sure that you want to create a new VCS root?` (title `Git Init`); runs `git init` with progress `Initializing repository...` / `Creating Git repository...`; success notification `Created Git repository in <dir>` (title `Git Integration Enabled`-style `<VCS> Integration Enabled`); failure `Could not git init <dir>` / `Git init failed`; the new root is registered; staging files fails with `Adding files to Git failed`. After init the editor offers `Add Files to Git` prompts per VCS-CHG-041 policies. | Init in a fresh project: `.git` exists, branch widget shows nothing until first commit (`Cannot create new branch in empty repository. Make initial commit first`). |
| VCS-SETUP-013 | SHOULD | `Enabling Git...` progress for the migration of an existing mapping; `Share Project on <hosting>` flows are in `11-git-hosting.md`. | Not applicable. |

## 3. Directory mappings and roots

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| VCS-SETUP-020 | MUST | Settings > Version Control (`project.propVCSSupport.Mappings`, page name `Version Control`, project scope, root group weight 45) shows `Directory Mappings`: a table with columns `Directory` and `VCS`, buttons Add/Edit/Remove; the project mapping row reads `<Project>` (tooltip `Content roots of all modules, and all immediate descendants of project base directory` or with the settings directory `... and <.idea> directory contents`); `Detected <path>` forms for detected roots; checkbox `Enable automatic mapping detection` (hint `Add and remove mapping on project structure changes. Detect newly created VCS roots.`); label `Unregistered roots:` with the list of directories not mapped; add/edit dialog `Add VCS Directory Mapping` / `Edit VCS Directory Mapping` / `Directory Mapping` (button `Save Mapping`, fields `Directory:` and `VCS:`; directory chooser `Select Directory` / `Select directory to map to a VCS`; status `Looking for VCS Administrative Area`; error `Invalid VCS options: <msg>`). Unknown VCS names show `<Unknown VCS> (<name>)`. | The default Git project shows one `<Project>` row. |
| VCS-SETUP-021 | MUST | Automatic root detection: on project open and when directories change, Git roots found under the content roots (a `.git` directory or file) are mapped when automatic detection is on; notifications: `Added Git root: <path>` (`Added Git roots: <list>`), title `Git Repository Found` / `Git Repositories Found` with action `Enable Integration` or `Configure...`/`Ignore`; `Git Integration Enabled`; unregistered roots are listed once with `Ignore`; invalid mappings: title `Invalid VCS root mapping(s)`, text `The directory <d> is registered as a Git root, but no Git repositories were found there.`, `VCS root configuration problems`, `The following directories are registered as VCS roots, but they are not:`. Ignoring stores the root in `IGNORED_UNREGISTERED_ROOTS`. | Open a project containing a nested `.git`: the notification offers to enable integration. |
| VCS-SETUP-022 | MUST | In `Safe mode` (untrusted project) VCS is unavailable: text `Safe mode, Version Control is not available. Trust the project to access full IDE functionality.`; toolbar separator `Safe mode, Version Control is not available`; Git commands refuse with `Cannot run a Git command in the safe mode`. | Untrusted project: the settings page shows the sentence. |
| VCS-SETUP-023 | SHOULD | Settings reachable by search: top-hit provider (`VcsOptionsTopHitProvider`) exposes checkboxes of the Version Control pages in Search Everywhere; the page `Confirmation` (`configurable.VcsGeneralConfigurationConfigurable.display.name`) and sub-pages `Changelists`, `Commit`, `Shelf`, `Issue Navigation`, `Git`, `Log`, `Background`... per `plugin.xml` registration; `Commit` sub-page: VCS-CMT checks. | Search `commit message` finds the Commit page. |

## 4. Ignore files

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| VCS-SETUP-030 | MUST | Ignore files (`.gitignore` and others) have a dedicated file type with syntax highlighting of comments, negation `!`, globs, directory markers; the inspection `Ignore file duplicates` (group `Version control`, message `<pattern> pattern is defined more than once`, quick fix `Remove pattern`) flags repeated entries. | A duplicated line is flagged with the message. |
| VCS-SETUP-031 | MUST | `Add to <ignore file>` actions in Project view/Changes view (VCS-CHG-045) create the file when missing (`Create <name>` / `Create <name> file in <dir>?`), append relative paths (directories with trailing `/`), refresh VCS state; Git extras: `Add to .git/info/exclude`, `Open .git/info/exclude` (`Git.OpenExcludeFile`). Ignored files drop out of Unversioned. | Add a build dir to `.gitignore`: it leaves Unversioned. |
| VCS-SETUP-032 | SHOULD | Managed ignoring (VCS-CHG-046) and the dialog `Add Files to Ignore` (button `Ignore`, view dialog) suggest files from `Default ignored files`, project exclude paths (`Project exclude paths`), build output; the Ignored settings panel (`Ignored Files` tab, `General settings`, `Excluded directory management policy`) lists the patterns; `Check Ignored but not Excluded Directories` (`CheckIgnoredAndNotExcludedDirectories`, internal-style action) reports directories that are ignored by VCS but not marked excluded, offering to exclude them. | A project with an ignored `node_modules` directory prompts to exclude it. |
| VCS-SETUP-033 | SHOULD | New-file flow: files created in the IDE that match ignore rules never trigger add prompts; files from `Add to VCS` that are ignored produce the confirmation `Add ignored files?`-style prompt via `ScheduleForAdditionWithIgnoredFilesConfirmationAction` (dialog lists ignored files; the user may force-add). | `Ctrl+Alt+A` on an ignored file asks for confirmation. |
| VCS-SETUP-034 | SHOULD | `Add to VCS` on files adds them with `git add` (progress `Adding files...`, error `Failed to Add Files` / `The following problems have occurred when adding the files:`), removal `Removing files...`; VFS listener prompts per VCS-CHG-041/042. | Verified through cases of VCS-CHG-04x. |

## 5. Status-bar, widgets and entry points of the VCS UI

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| VCS-SETUP-040 | MUST | Tool windows: `Commit` (anchor left, id `Commit`, shortcut `Alt+0`), `Version Control` (bottom, id `Version Control`, toolbar label action `VcsToolbarLabelAction` `VCS Label`; tabs Log, Local Changes/Shelf when the commit tool window is off, Console), `Git` log is the `Log` tab. Registry `vcs.empty.toolwindow.show` (default on) shows the Version Control tool window even without VCS (empty state text per VCS-CHG-030). Tool window title `Version Control`. | Fresh Git project: stripe has Commit on the left and Git/Version Control at the bottom. |
| VCS-SETUP-041 | SHOULD | Console tab: `Show VCS Console Tab` (`Vcs.ShowConsoleTab`, tab title `Console`) lists executed VCS commands with output (notification `Show details in console`). | After a fetch the console lists `git fetch` lines. |
| VCS-SETUP-042 | SHOULD | `Vcs.Show.Toolwindow.Tab` group: `Show Local Changes`, `Show Shelf`, `Show VCS Log`, `Show Git Stash`; `Version Control` main menu item named `Git` (group text `_Git`; with several VCS `VCS`) containing: Commit, Push, Update Project, Pull, Fetch, Unshallow, Merge, Rebase, Resolve Conflicts, Revert Resolved, Branches, New Branch, New Tag, Reset HEAD, Worktrees, Show VCS Log, Patch, Uncommitted Changes, Git (file actions: Annotate, Compare..., Show History), Manage Remotes, Clone, VCS Operations Popup, Revert/Cherry-pick state actions. | Menu order matches `Git.MainMenu` (VCS-BR items). |

## Open questions

- OQ-1: the clone dialog layout in the new welcome screen (sidebar, field order, `Clone into:`
  vs `Directory:`) differs by surface; verify the field labels against a screenshot.
- OQ-2: default clone destination directory name per OS (`IdeaProjects` vs `projects`) comes from
  the application's path settings; align with the project-creation spec when it exists.
- OQ-3: which hosting clone pages are present by default depends on bundled plugins (GitHub,
  GitLab); see `11-git-hosting.md`.

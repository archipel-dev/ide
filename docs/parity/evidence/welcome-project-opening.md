# Welcome and project opening

This is a partial slice of issue #21, not full welcome-screen parity.

Launch without arguments:

```sh
ARCHIPEL_NO_UPDATE_CHECK=1 cargo run -p archipel-ide-app
```

The welcome surface offers Open Project. Click it, enter an existing directory's
full path and press Enter or Open. The workbench should replace the welcome screen,
show the project tree and update the window title. Opening is also available from
File, the project widget, and Cmd+O / Ctrl+O.

Enter a nonexistent path: the dialog must remain open and show an error. Correct
the path and retry. Escape and Cancel dismiss it.

Open a text file, edit it, and switch projects: the first project's dirty files
and workspace are saved before switching. Reopen the first project to restore
tabs. Invalid targets and failed document saves preserve the current editor.
A running build/run task must be stopped before switching.

Automated evidence: `cargo test -p archipel-ide-core switching_projects` and
`cargo test -p archipel-ide-core failed_project_open` exercise these engine policies
through the same typed `project.open` command used by the UI.

Native folder selection, persistent recent projects, New Project generators,
cloning, customization, plugins, and trust prompts remain pending.

Manual macOS validation (2026-10-10, native development preview): observed welcome
screen, Open button, Cmd+O, inline error for a nonexistent directory, successful
retry, Enter submission, transition to the project tree, updated window title and
left-aligned toolbar. Native accessibility reports close, minimize and full-screen
controls. No pixel-level comparison or full-screen layout verification is claimed.

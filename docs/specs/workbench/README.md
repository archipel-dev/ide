# Workbench specification (clean-room)

Requirements for the Archipel IDE workbench: tool windows, Project view, Structure / Problems / TODO / Bookmarks, status bar, settings and keymap, welcome and project entry, notifications and background tasks, file operations. Source basis: IntelliJ IDEA Community (`ic:` citations), format as in `docs/specs/codeinsight/README.md`. Executable cases are in `cases/`.

| File | Topic | ID prefixes | MUST | SHOULD | LATER | Cases |
|---|---|---|---|---|---|---|
| [01-tool-window-framework.md](01-tool-window-framework.md) | Tool window framework | `WB-TW` | 42 | 15 | 3 | 37 |
| [02-project-view.md](02-project-view.md) | Project view | `WB-PV` | 48 | 16 | 2 | 63 |
| [03-structure-problems-todo.md](03-structure-problems-todo.md) | Structure, Problems, TODO, Bookmarks | `WB-BKM`, `WB-PRB`, `WB-STR`, `WB-TODO` | 42 | 12 | 4 | 52 |
| [04-status-bar.md](04-status-bar.md) | Status bar and widgets | `WB-SB` | 28 | 10 | 1 | 46 |
| [05-settings-dialog-and-keymap.md](05-settings-dialog-and-keymap.md) | Settings dialog and Keymap | `WB-KEY`, `WB-SET` | 34 | 9 | 2 | 41 |
| [06-welcome-new-project-open.md](06-welcome-new-project-open.md) | Welcome, New Project, Open, trust | `WB-WEL` | 31 | 8 | 0 | 40 |
| [07-notifications-and-background-tasks.md](07-notifications-and-background-tasks.md) | Notifications and background tasks | `WB-NOT` | 29 | 7 | 1 | 37 |
| [08-file-operations-and-scratches.md](08-file-operations-and-scratches.md) | File operations, scratches, templates, encoding | `WB-FILE` | 27 | 10 | 2 | 37 |
| **Total** | | | **281** | **87** | **15** | |

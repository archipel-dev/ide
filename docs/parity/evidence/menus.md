# Main menu (scene 07)

Reference: `reference/intellij-community/java-gradle/07-main-menu.png`. Archipel IDE:
`archipel-ide/menus/07-file-menu.png`, `07-edit-menu.png`, `07-code-menu.png` (Linux, Xvfb
1600×1000, XWin keymap).

## Steps

1. Click the hamburger. As in the New UI, the titles File, Edit, View, Navigate, Code,
   Refactor, Build, Run, Tools, Git, Window and Help expand inline in the main toolbar, in
   place of the project and VCS widgets.
2. Click File → `07-file-menu.png`. Hovering another title switches menus →
   `07-code-menu.png`, `07-edit-menu.png`.
3. Esc, a click outside, or running an item collapses the bar back to the hamburger.

## Matches the reference

- The File menu has the reference order and separators: New ▸, Open…, Recent Projects ▸,
  Close Project | Settings… (Ctrl+Alt+S), Project Structure… (Ctrl+Alt+Shift+S),
  File Properties ▸ | Local History ▸ | Save All (Ctrl+S), Reload All from Disk
  (Ctrl+Alt+Y), Invalidate Caches… | Manage IDE Settings ▸ … | Export ▸, Print… |
  Power Save Mode | Exit.
- The Code menu, which was an empty popup before (`archipel-ide/NOTES.md`, 07), now lists
  the reference structure. Code Completion ▸, Comment with Line Comment and Move Line
  Up/Down are enabled.
- Shortcuts come from the active keymap.

## Still differs

- No menu icons, no underlined mnemonics.
- The reference build shows "Repair IDE" and a "New Projects Setup" submenu. The spec
  data (build 262) inlines Settings for New Projects… and the run-configuration
  templates instead.
- Entries Archipel IDE does not implement are shown disabled with the tooltip "Not available
  in Archipel IDE yet". The reference hides some of them at runtime. Only the five File
  entries hidden in the reference screenshot are hidden.
- Archipel IDE-only commands (Refresh Plugins, Run, Stop, Refresh Git Status, terminal
  actions, update actions) are appended after a separator in their menu, because the
  reference has no matching action or registers it at runtime (Run).

# Context menus (scene 19)

Reference: `reference/intellij-community/java-gradle/19-project-context-menu.png`. Archipel IDE:
`archipel-ide/menus/19-project-context-menu.png`, `tab-context-menu.png`,
`editor-context-menu.png`.

The menus are built from `ProjectViewPopupMenu`, `EditorTabPopupMenu` and
`EditorPopupMenu`. Like the reference's popups, they show only the actions available
for the clicked target, in reference order and grouping:

- Project view (`Cargo.lock`): New ▸ | Copy Path/Reference… ▸, Edit Source (F4) |
  Find in Files… | Rename… (Shift+F6) | Delete… (Delete). The project root has no
  Rename, Delete or Edit Source. Escape closes the menu (scene 19b bug).
- Editor tab: Close Tab (Ctrl+F4), Close Other Tabs, Close All Tabs, Close Tabs to the
  Left/Right (disabled when there are none) | Copy Path/Reference… ▸.
- Editor: Cut, Copy, Paste | Find in Files.

Missing compared with the reference (scene 19): Cut/Copy/Paste of files, Find Usages,
Analyze, Refactor, Bookmarks, Reformat Code, Optimize Imports, Override File Type,
Run/Debug, Open in Right Split, Open In, Local History, Git, Repair IDE on File, Reload
from Disk, Compare With. They appear automatically once their actions are implemented
and mapped.

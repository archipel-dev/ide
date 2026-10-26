# Editor splits and the preview tab

Captured 2026-10-11 on Linux under Xvfb (1600x1000, `ARCHIPEL_RENDERER=glow`, debug build),
driven through the control socket (`archipel-cli ui`). Sample project: `src/App.java`,
`src/Util.java`, `README.md`, `build.gradle.kts`.

| File | Shows |
|---|---|
| `audit-v0.0.5/04-fixed-preview-tab.png` | single click on `App.java` in the Project tree opens an italic preview tab (AUD-034/IJ-ED-161); the next single click replaces it, a double click or an edit keeps it |
| `audit-v0.0.5/04-fixed-tab-menu-splits.png` | tab context menu with Split Right, Split Down, Move to Opposite Group, Change Splitter Orientation, Unsplit, Unsplit All, Keep Tab Open (disabled entries are shown dimmed) |
| `audit-v0.0.5/04-fixed-editor-splits.png` | three groups (`App.java` over `Util.java` on the left, `README.md` on the right), each with its own tab strip; the focused group (blue divider) is the one whose tab the status bar and title show |
| `audit-v0.0.5/07-fixed-window-editor-tabs.png` | Window > Editor Tabs lists Split Right / Split Down / Change Splitter Orientation / Unsplit / Unsplit All, wired through `ACTION_COMMANDS` |
| `audit-v0.0.5/19-fixed-open-in-right-split.png` | Project tree context menu with "Open in Right Split  Shift+Enter" |
| `audit-v0.0.5/05-fixed-search-everywhere-right-split.png` | Search Everywhere: typing `build.gr` and pressing Shift+Enter opened `build.gradle.kts` in the right group |

Behavior notes:

- A tab lives in exactly one group, so Split Right / Down *move* the tab into the new group
  (an empty group when it is the group's only tab). Showing one document in two groups is
  not implemented.
- Open in Right Split targets the group after the focused one, wrapping around, and
  creates a split to the right when the editor is not split.
- Dragging a tab onto the outer quarter of an editor group splits at that edge; dropping on
  another group moves the tab. The control socket has no drag primitive, so this path is
  covered by `editor.splitAt` / `editor.moveToGroup` tests in `archipel-ide-core` and the pure
  `archipel_ide_split::drop_edge` test.
- Splits are stored in `.archipel/workspace.json` (`splits`) and restored with the tabs.

## Real mouse and keyboard run (1280x800)

Captured under Xvfb at 1280x800 with `xdotool` only (real clicks, double clicks, drags,
`Shift+Return`, window focus set with `xdotool windowfocus`); no command dispatch. All
UI stays inside the window.

| File | Shows |
|---|---|
| `audit-v0.0.5/04-fixed-mouse-1280-preview-tab.png` | single click in the tree: italic preview tab |
| `audit-v0.0.5/04-fixed-mouse-1280-tab-drag-highlight.png` | mouse-down on a tab and drag towards the right edge: the drop zone is highlighted |
| `audit-v0.0.5/04-fixed-mouse-1280-tab-drag-split.png` | after the release the tab sits in a new right group |
| `audit-v0.0.5/04-fixed-mouse-1280-tab-context-menu.png` | right click on a tab, hover on "Split Down" |
| `audit-v0.0.5/04-fixed-mouse-1280-divider-resized.png` | divider dragged left with the mouse |
| `audit-v0.0.5/04-fixed-mouse-1280-search-everywhere-shift-enter.png` | Search Everywhere opened with the toolbar icon; Shift+Return then opened the hit in the opposite group |
| `audit-v0.0.5/04-fixed-mouse-1280-tree-shift-enter.png` | tree row clicked, Shift+Return: the file moved to the opposite group |

Bugs found by this run and fixed:

- Replacing a preview tab inside a split left the previous tab active (the close ran
  before the split tree knew about the new tab); regression test
  `replacing_a_preview_in_a_split_keeps_the_new_tab_active`.
- A single click in the tree took the keyboard to the editor, so Enter / Shift+Enter edited
  the file; the tree now keeps keyboard focus after a preview click.
- Shift+Return in the tree was missed when Shift was released within the same frame; the
  key event's own modifiers are used.

The same drags were repeated at 1440x900 (`04-fixed-mouse-1440-tab-drag-highlight.png` edge drop
zone, `04-fixed-mouse-1440-tab-drag-to-group-highlight.png` drop zone over another group,
`04-fixed-mouse-1440-tab-moved-to-group.png` result: `README.md` moved into the right group).

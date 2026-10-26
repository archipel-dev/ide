# Workbench (main window) specification

Target: IntelliJ IDEA 2026.2 (build 262.x), New UI, Islands theme (Islands Dark / Islands Light).

> Exact colors, arcs, border widths and sizes come from the theme data in
> [../themes.md](../themes.md) and [../data/themes.json](../data/themes.json). Where this
> document disagrees with that data, the data wins. Pixel values marked "(inferred)" here
> need to be measured in a real install.

Status: research spec, compiled 2026-10-10 from public JetBrains documentation, JetBrains
blog posts and the IntelliJ Platform UI Guidelines. Nothing here is parity evidence; parity
claims still require reproducible evidence in `docs/parity/`.

## 0. How to read this document

- Everything below is written in our own words. Quoted strings are UI labels (menu items,
  setting names, button captions) that we must reproduce for user familiarity.
- **(inferred)** marks a statement we did not read in a public source. It comes from general
  knowledge of the product, from screenshots on linked pages, or from reasoning about
  consistency. Every inferred value that affects pixels or defaults also appears in
  section 9 (Open questions) so a human can confirm it in a real 2026.2 install.
- Shortcuts are given as `Win/Linux` and `macOS` in the default keymaps ("Windows" and
  "macOS"). Where a source only gave one platform, the other is marked (inferred).
- Clean-room: no screenshots, icons or artwork are embedded. Where a screenshot helps, we
  link the public page that shows it and describe what is visible.
- Implementation mapping hints for archipel-ide are given in "Impl notes" blocks. These are
  suggestions; the architecture docs win on conflict.

### Global terminology

| Term (ours) | Product term | Meaning |
|---|---|---|
| Frame | Project window / main window | One OS window per open project. |
| Header | Main toolbar / window header | Top strip containing menu, project, VCS, run widgets. |
| Stripe | Tool window bar | Vertical icon strip on the left or right edge. |
| Island | Island | A rounded panel (editor area, each docked tool window group) floating on the main window background. |
| Tool window | Tool window | Dockable panel (Project, Terminal, ...). |
| Editor area | Editor | Tabbed editor splitters in the center. |
| Status bar | Status bar | Bottom strip with breadcrumbs (navigation bar) and widgets. |

---

## 1. Window anatomy

### 1.1 Overall layout (top to bottom)

```
+--------------------------------------------------------------------------------+
| HEADER: [menu] [project v] [branch v]   ...drag area...  [run cfg v][>][bug][:] [search][gear][AI] [win ctrls] |
+----+------------------------------------------------------------------+----+
| L  | +-----------+ +------------------------------------+ +---------+ | R  |
| S  | | left TW   | | EDITOR ISLAND (tabs, breadcrumbs?, | | right TW| | S  |
| T  | | island    | |  gutter, text, scrollbar)          | | island  | | T  |
| R  | +-----------+ +------------------------------------+ +---------+ | R  |
| I  | +--------------------------------------------------------------+ | I  |
| P  | | BOTTOM TW ISLAND (Terminal, Run, Problems...)                | | P  |
| E  | +--------------------------------------------------------------+ | E  |
+----+------------------------------------------------------------------+----+
| STATUS BAR: [navigation breadcrumbs ...........]  [progress] [12:5] [LF] [UTF-8] [4 spaces] [lock] |
+--------------------------------------------------------------------------------+
```

- The header spans the full window width. On Windows and Linux it replaces the OS title bar
  (custom-decorated frame with the window controls at its right end). On macOS the header
  is merged with the native title bar, and the traffic-light buttons sit at its left end
  (inferred from the New UI screenshots on the new-ui help page).
- Left and right stripes run from below the header to above the status bar.
- Between the stripes are the islands: the editor island in the middle, tool-window islands
  around it. The space between islands shows the main window background color.
- The status bar spans the full width at the bottom.
- The guided-tour help page numbers nine main-window parts: navigation bar, Project tool
  window, editor, popup menu, context menu, status bar, tool window bar, toolbar, gutter.
  Its screenshot is the best public reference for the overall composition.

### 1.2 Header (main toolbar)

The header has three logical zones: left, center (drag area) and right. Default content, in
order:

| # | Zone | Element | Default visible | Behaviour |
|---|---|---|---|---|
| 1 | Left | Main menu | Win/Linux only | See 1.3. On macOS the native global menu bar is used and nothing is drawn here. |
| 2 | Left | Project widget | Yes | Shows the current project name with a chevron. Click opens a popup to switch to recent projects, create a new project, or open one. With "Use project colors in main toolbar" on, the header's left part is tinted with a per-project color (see 1.6). |
| 3 | Left | VCS widget | Yes, when the project is under VCS | Shows the current branch name with a chevron. Click opens the Branches popup (6.12). Shows a blue incoming arrow when the remote has unfetched commits and a green outgoing arrow when local commits are unpushed. When the project has no VCS, it shows a "Version Control" entry offering to create a repository (inferred). |
| 4 | Center | Empty drag area | Yes | Drags the window. Double-click maximizes/restores (inferred: follows OS convention). |
| 5 | Right | Run widget | Yes | Run configuration selector + Run + Debug + "More" (kebab) buttons. See 1.4. |
| 6 | Right | Search Everywhere button | Yes (inferred) | Magnifier icon. Opens Search Everywhere (6.1). |
| 7 | Right | IDE and Project Settings button | Yes | Gear icon. The help pages call it "IDE and Project Settings". Opens a menu with Settings, Project Structure, Plugins, Theme, Keymap, View Mode etc. (menu contents inferred). |
| 8 | Right | AI widget(s) | When AI Assistant / an agent plugin is installed (inferred) | Icon that opens the AI chat tool window and shows quota state. Exact placement not documented (see Open questions). |
| 9 | Right | Window controls | Win/Linux | Minimize, maximize/restore, close. |

Additional rules:

- The classic toolbar (Open, Save, Undo, Redo...) is hidden by default and enabled via
  "View | Appearance | Toolbar". In the New UI it is drawn as a row below the header
  (inferred).
- Right-click on the header offers "Add Action to Main Toolbar" (a submenu of popular
  actions) and "Customize Toolbar…" which opens the "Customize Main Toolbar" dialog. The
  dialog shows a tree of toolbar sections with "Add", "Remove", "Edit Icon", "Move Up",
  "Move Down" and "Restore" buttons. The same tree is reachable under
  Settings | Appearance & Behavior | Menus and Toolbars.
- The window title text (project name, current file name, module name if the project has
  several modules) is shown by the OS on macOS and in the taskbar on Windows. "Always show
  full paths in window header" (Appearance) switches the file part to a full path.
- Header widgets are flat buttons: no border at rest, a rounded hover background, a
  stronger pressed background (inferred, same as toolbar buttons in 8.4).

Impl notes: model the header as an ordered list of `HeaderItem` descriptors in three
groups (`left`, `center`, `right`), each bound to a command id. "Customize Main Toolbar"
edits that list, and it persists at application level.

### 1.3 Main menu (Windows/Linux)

Setting: Settings | Appearance & Behavior | Appearance → "Main menu", also
"View | Appearance | Main Menu". Three options:

| Option label | Behaviour |
|---|---|
| "Hide under Hamburger Button" | A single hamburger icon at the far left of the header. Click (or `Alt+\`) opens a vertical popup listing top-level menus (File, Edit, View, Navigate, Code, Refactor, Build, Run, Tools, Git, Window, Help). Each item opens its submenu as a cascading popup. Default (inferred; see Open questions). |
| "Merge with Main Toolbar" | The top-level menu names are drawn inline in the header, left of the project widget. Added in 2025.1. If the header lacks room, overflowing items collapse into a hamburger (inferred). |
| "Show above Main Toolbar" | A classic full-width menu bar is drawn as its own row above the header. |

- Pressing `Alt` alone focuses the menu, and mnemonics work if "Enable mnemonics in menu" is on.
- On macOS this setting is absent. The native menu bar uses the "IntelliJ IDEA" application
  menu, which holds Settings (`⌘,`).

### 1.4 Run widget

- Layout, left to right: configuration selector (icon of the configuration type + name +
  chevron), Run button (green triangle), Debug button (bug), "More" button (vertical
  ellipsis) (inferred ordering from New UI screenshots).
- When no configuration exists, the selector shows "Current File". Run/Debug then act on the
  file in the active editor. The expanded "Current File" menu offers debug, run with
  coverage, profile and run-configuration editing.
- Selector popup: a list of configurations, each row with its own Run button on hover.
  `Ctrl`+click on several rows runs them together. Temporary configurations are drawn with
  semi-transparent icons, permanent ones are opaque, and a configuration with an error gets
  a red cross. A temporary configuration's "⋯" menu offers "Save Configuration". Items can
  also be pinned to the top or deleted, and the popup ends with "Edit Configurations…"
  (inferred label, matches the Run menu action).
- While a process runs: the Run button becomes "Rerun" (circular arrow), and a Stop button
  (red square) appears. With several running processes, Stop opens a chooser (`Ctrl+F2`
  "Stop" with several processes asks which one). The widget background is tinted green
  while running and orange-ish while debugging (inferred: observed New UI behaviour; verify).
- Shortcuts:

| Action | Win/Linux | macOS |
|---|---|---|
| Run selected configuration / rerun | `Shift+F10` | `⌃R` |
| Debug selected configuration | `Shift+F9` | `⌃D` |
| Run… (choose configuration popup) | `Alt+Shift+F10` | `⌃⌥R` |
| Debug… (choose configuration popup) | `Alt+Shift+F9` (inferred) | `⌃⌥D` |
| Stop | `Ctrl+F2` | `⌘F2` |
| Edit Configurations (via Run… then `0`) | `Alt+Shift+F10`, `0` | `⌃⌥R`, `0` (inferred) |

### 1.5 Compact mode

- Toggle: "View | Appearance | Compact Mode", or the "Compact mode" checkbox under
  Settings | Appearance & Behavior | Appearance → UI Options. Default: off.
- Effect: lower header and tool-window header heights, tighter spacing and padding, smaller
  icons and buttons. Nothing is hidden.
- Islands geometry also has compact variants (see 1.7).

Proposed dimensions (all **inferred**, need measurement):

| Element | Normal | Compact |
|---|---|---|
| Header height | 40 px | 32 px |
| Stripe width | 40 px | 32 px |
| Stripe button (hit area) | 32×32 px with 20×20 icon | 24×24 px with 16×16 icon |
| Tool window header height | 40 px | 32 px |
| Editor tab height | 40 px | 32 px |
| Status bar height | 30 px | 24 px |

### 1.6 Window modes

| Mode | Enter/exit | Hides |
|---|---|---|
| Full Screen | View | Appearance → "Enter Full Screen" / "Exit Full Screen" | OS chrome. On macOS the menu bar slides in when the pointer touches the top edge. |
| Distraction-free | View | Appearance → "Enter Distraction Free Mode" | Tool windows, toolbars, editor tabs. Text is centered, and its width comes from "Hard wrap at" (Code Style). Advanced Settings has "Left margin in distraction free mode". |
| Presentation | View | Appearance → "Enter Presentation Mode" | Everything except the editor. Font is enlarged ("Presentation Mode → Zoom" in Appearance). |
| Zen | View | Appearance → "Enter Zen Mode" | Full Screen plus Distraction-free combined. |
| Compact | see 1.5 | Nothing. |

- All modes are also listed under "View | Quick Switch Scheme" (`Ctrl+\``, macOS `⌃\``),
  sub-item "View Mode".
- None of these modes has a default shortcut. The macOS full-screen shortcut `⌃⌘F` is
  inferred from the OS convention.
- "Use project colors in main toolbar" (UI Options) tints the header per project. The
  color is chosen from the project widget popup (inferred).

### 1.7 Islands look

Islands has been the default look since 2025.3. It is a visual layer only, so behaviour is
unchanged. Bundled variants: "Islands Dark", "Islands Light", "Islands Darcula".

Visual rules:

- The editor area and each docked tool-window area are separate rounded rectangles
  ("islands") on a distinct main-window background. In dark Islands themes the main
  background is **lighter** than the islands. In light themes it is **darker**. The
  platform recommends a contrast of at least 1.20:1 between the islands and the main
  background.
- The stripes, header and status bar sit directly on the main background, so their own
  borders are transparent in Islands (`StatusBar.borderColor`,
  `ToolWindow.Stripe.borderColor` and `MainToolbar.borderColor` are transparent).
- The island border color equals the tool window background, so islands have no visible
  outline. Separation comes from the gap and the background contrast.
- Tool window borders are easier to grab for resizing, because the gaps are wider than the
  old 1 px splitters.
- Selected tabs, including editor tabs, get a **filled background** in addition to (or in
  place of) the classic underline. Theme keys: `EditorTabs.underlinedTabBackground`,
  `EditorTabs.inactiveUnderlinedTabBackground`, `EditorTabs.underlinedBorderColor`,
  `EditorTabs.inactiveUnderlinedTabBorderColor`. `EditorTabs.background` matches the editor
  background.
- When the IDE window is not focused, the islands are dimmed using `Island.inactiveAlpha`.
  The SDK docs mention 0.44, but the 2026.2 Islands Dark and Light theme data sets 0.56
  (see [../themes.md](../themes.md)). The theme data takes precedence.
- Setting "Different tool window background" (Appearance, shown only for Islands themes)
  makes tool-window islands lighter than the editor in dark themes and darker in light
  themes, so the editor stands out.

Geometry (from the platform SDK "Supporting Islands theme" page):

| Key | Default | Rendered meaning |
|---|---|---|
| `Island.arc` | 20 | Stored value is twice the radius, so the corner radius is **10 px**. |
| `Island.arc.compact` | 16 | **8 px** radius in compact mode (inferred by the same halving rule). |
| `Island.borderWidth` | 5 | Gap between islands = value − 1 = **4 px**. |
| `Island.borderWidth.compact` | 4 | **3 px** gap in compact mode (inferred by the same rule). |
| `Island.inactiveAlpha` | 0.44 | Inactive-window dim overlay. |

Impl notes: encode these as theme tokens (`island.radius`, `island.gap`,
`island.inactive_alpha`, `window.background`, `toolwindow.background`, `editor.background`).
The layout engine computes island rectangles first and then insets content by the radius
where needed (for example scrollbars must not overlap the rounded corners; inferred).

Sources:
- https://www.jetbrains.com/help/idea/guided-tour-around-the-user-interface.html
- https://www.jetbrains.com/help/idea/new-ui.html
- https://www.jetbrains.com/help/idea/settings-appearance.html
- https://www.jetbrains.com/help/idea/customize-actions-menus-and-toolbars.html
- https://www.jetbrains.com/help/idea/running-applications.html
- https://www.jetbrains.com/help/idea/run-debug-configuration.html
- https://www.jetbrains.com/help/idea/manage-branches.html
- https://www.jetbrains.com/help/idea/ide-viewing-modes.html
- https://www.jetbrains.com/help/idea/reference-keymap-win-default.html
- https://www.jetbrains.com/help/idea/reference-keymap-mac-default.html
- https://www.jetbrains.com/idea/whatsnew/2025-1/ (merged main menu)
- https://www.jetbrains.com/idea/whatsnew/2025-3/ (Islands default)
- https://blog.jetbrains.com/platform/2025/09/islands-theme-the-new-look-coming-to-jetbrains-ides/
- https://blog.jetbrains.com/platform/2025/12/meet-the-islands-theme-the-new-default-look-for-jetbrains-ides/
- https://plugins.jetbrains.com/docs/intellij/supporting-islands-theme.html

---

## 2. Tool windows

### 2.1 Stripes (tool window bars)

- There are two stripes, left and right. The New UI has no bottom stripe. Bottom tool windows
  are opened from the **lower** group of the left or right stripe.
- Each stripe has two groups separated by a gap or separator line:
  - **Upper group**: buttons for *vertical* tool windows (docked to that side).
  - **Lower group**: buttons for *horizontal* tool windows (docked to the bottom). Lower-group
    buttons on the left stripe open in the bottom-left part of the bottom area, and lower-group
    buttons on the right stripe open in the bottom-right part (inferred from the split model).
- Within each group, dragging a button past a separator creates a **split**. Two tool
  windows from the same side then show at the same time, stacked (for vertical windows) or
  side by side (for horizontal ones). The new-ui page describes this as dragging icons "to
  arrange vertical or horizontal splits".
- Buttons are icons only, by default. Right-click a stripe → "Show Tool Window Names" adds a
  small label under each icon (default off). The Appearance settings have "Show tool window
  bars" (default on) and "Show tool window names".
- Hovering a button shows a tooltip with the name and shortcut, for example "Project (Alt+1)"
  (format inferred).
- States: idle (monochrome icon), hover (rounded background), **active/open** (filled rounded
  background, persists while the window is visible), focused (the same as active, plus the
  window header shows focus) (states inferred from the UI guidelines and screenshots).
- Badge: a tool window can show a small colored dot over its icon when its content changes
  (for example errors or updates). The icon itself must not change.
- "More tool windows" button: an ellipsis (`⋯`) button at the end of the left stripe's upper
  group (inferred placement). It opens a popup listing every available tool window that
  has no stripe button, plus all others alphabetically (inferred). Choosing one opens it in
  its default location.
- Context menu on a stripe button (labels from Project tool window options and the arrange
  page): "Hide" (only when open), "Move to" ▸ (target positions, see 2.4), "Remove from
  Sidebar", "View Mode" ▸, "Resize" ▸, "Show Tool Window Names" (inferred to be shared
  with the stripe background menu).
- Click on a button toggles: if hidden, show and focus it; if visible but not focused,
  focus it (inferred); if visible and focused, hide it. A tool window shortcut
  (e.g. `Alt+1`) behaves the same.
- Drag a button onto another stripe or group to move the tool window. Dragging the tool
  window header also moves it (header dragging can be turned off in Advanced Settings).
- "Drag-and-drop with Alt pressed only" (Appearance, default off) requires `Alt` for these
  drags.

Icon sizes: the guidelines ask plugin authors for stripe icons at 20×20 (normal) and 16×16
(compact). Icons are grey and monochrome.

### 2.2 Tool window anatomy

```
+-----------------------------------------------------------------+
| Title  [Tab1] [Tab2 x] [+]  ⌄           [tb actions] [⋮] [—]   |  <- header
+-----------------------------------------------------------------+
| optional toolbar (vertical on the left for horizontal windows) |
| content                                                         |
+-----------------------------------------------------------------+
```

- **Title**: the tool window name in title case, ideally at most two words.
- **Tabs / content selector**: tool windows with several contents (Run sessions, Find
  results, Project views) show them as header tabs. If "Group Tabs" is on (Options menu), they
  show as a single dropdown next to the title instead. The Project tool window shows its
  view selector ("Project", "Packages", "Project Files", "Open Files"...) this way.
  `Alt+Right` / `Alt+Left` cycle tabs, and `Alt+Down` lists them.
- Tabs for similar entities (sessions, results) are closable. A "+" button adds a new one
  where it makes sense (Terminal).
- **Header actions** (right side): window-specific actions such as "Expand All",
  "Collapse All", "Select Opened File", then the **Options** menu (`⋮`, three vertical
  dots), then **Hide** (a minimize dash `—`).
- Header actions are shown only while the pointer hovers the window or it has focus. The
  Advanced Settings option "Always show tool window header icons" keeps them visible.
- Right-clicking the header opens the same menu as Options.
- **Toolbar placement** follows the guidelines: vertical tool windows (trees) use a
  horizontal toolbar at the top. Horizontal tool windows use a vertical toolbar on the left,
  except for process controls or wide widgets such as search fields. The toolbar border is
  hidden while content is scrolled to the top and appears once content scrolls under it.
- **Empty state**: a window with no content shows centered guidance text with action links.
- Focus: a focused tool window gets an active header background. Unfocused ones use
  `ToolWindow.Header.inactiveBackground`.

Options menu (union of items documented for the Project tool window; window-specific items
come first):

| Section | Items |
|---|---|
| Window-specific | e.g. "Enable Preview Tab", "Open Files with Single Click", "Always Select Opened File", Appearance ▸ ..., "Sort By" ▸ ... |
| Generic | "Speed Search" (`Ctrl+F`), "Group Tabs", "View Mode" ▸, "Move to" ▸, "Resize" ▸, "Remove from Sidebar", "Hide" (`Shift+Escape`), "Help" |

### 2.3 View modes

Menu: "Window | Active Tool Window | View Mode", or Options → "View Mode".

| Mode | Attached to stripe | Behaviour |
|---|---|---|
| "Dock Pinned" (default) | Yes | Takes layout space next to the editor. Stays open when it loses focus. |
| "Dock Unpinned" | Yes | Takes layout space while active. Hides automatically when focus moves elsewhere. |
| "Undock" | Yes | Overlays the editor and other windows when active, without taking layout space. Hides when another tool window activates. |
| "Float" | No (still owned by the frame) | A borderless child window above the main frame. It can be moved to another monitor and is visible only while the project frame is open. |
| "Window" | No | A full separate OS window with its own title bar, independent of the project frame. |

### 2.4 Positions, splitting, resize, maximize

- "Move to" targets (inferred labels, derived from the stripe model): "Left Top",
  "Left Bottom", "Bottom Left", "Bottom Right", "Right Top", "Right Bottom". Top groups are
  vertical windows on that side, and bottom groups are horizontal windows in that half of
  the bottom area.
- Resize: drag the gap between islands. Keyboard ("Window | Active Tool Window | Resize"):

| Action | Win/Linux | macOS |
|---|---|---|
| Stretch to Left | `Ctrl+Alt+Shift+Left` | `⌃⌥⇧←` (inferred) |
| Stretch to Right | `Ctrl+Alt+Shift+Right` | `⌃⌥⇧→` (inferred) |
| Stretch to Top | `Ctrl+Alt+Shift+Up` | `⌃⌥⇧↑` (inferred) |
| Stretch to Bottom | `Ctrl+Alt+Shift+Down` | `⌃⌥⇧↓` (inferred) |
| Maximize Tool Window | `Ctrl+Shift+'` | `⌘⇧'` (inferred) |

- "Remember size for each tool window" (Appearance → Tool Windows): when off, all windows
  on a side share one width or height.
- "Widescreen tool window layout": vertical windows take the full height, and the bottom
  area is limited to the space between them.
- "Side-by-side layout on the left" / "... on the right": the top and bottom groups of a
  side show as two columns instead of stacking.

### 2.5 Hide / restore / focus

| Action | Win/Linux | macOS | Notes |
|---|---|---|---|
| Hide Active Tool Window | `Shift+Escape` | `⇧⎋` | |
| Hide All Tool Windows / Restore Windows | `Ctrl+Shift+F12` | `⌘⇧F12` | Toggle. Restore brings back exactly the set hidden by the previous Hide All. |
| Jump to Last Tool Window | `F12` | `F12` | Focuses the last active tool window. |
| Focus editor | `Escape` | `⎋` | From any tool window, returns focus to the last editor caret. |
| Open selected item in editor | `F4` | `⌘↓` (inferred) | Works in Find, Problems, Bookmarks... |
| Restore Default layout | `Shift+F12` | `⇧F12` (inferred) | Applies Window | Layouts | Default. |

- Advanced Settings: "Perform 'Hide All Tool Windows' / 'Restore Windows' with double-click
  on editor tab" (when on, double-clicking an editor tab toggles hide-all). Separately,
  double-clicking a tab maximizes the editor (same as `Ctrl+Shift+F12`) according to the
  editor-basics page. Treat both as the same toggle (inferred).

### 2.6 Layouts

"Window | Layouts" menu:

- "Default": a built-in layout that cannot be edited or deleted. `Shift+F12` applies it.
- "Save Changes in Current Layout": overwrites the current custom layout.
- "Save Current Layout as New…": prompts for a name.
- One submenu per saved layout with "Apply", "Rename…", "Delete" (the current layout cannot
  be deleted). The active layout has a check mark.
- Also reachable through "Quick Switch Scheme" (`Ctrl+\``) → "Layout".
- A layout stores, per tool window: anchor, group (top/bottom), order, split, visibility,
  view mode, size (inferred from "positions and custom sizes").

Persistence: layout state is per project (workspace file), and named layouts are
application-wide (inferred).

### 2.7 Default tool window placement (IntelliJ IDEA)

Availability rules (documented): Project and Structure are always available. Maven, Gradle
and Spring appear only when the project has matching configuration or plugins. Run, Debug
and Find appear when an action first needs them. Windows that do not apply to the project
must not show a stripe button. Windows with general value (Version Control, Problems) show
a button even when empty, and the rest go to "More tool windows".

Placement columns are **(inferred)** from New UI defaults and screenshots unless the note
says otherwise. Shortcuts are documented.

| Tool window | Default anchor / group | Orientation | Win/Linux | macOS | Stripe button by default? |
|---|---|---|---|---|---|
| Project | Left, upper | Vertical | `Alt+1` | `⌘1` | Yes |
| Commit | Left, upper (documented as a vertical window on the left) | Vertical | `Alt+0` | `⌘0` | Yes, when VCS is enabled |
| Pull Requests | Left, upper | Vertical | — | — | When the repo is on GitHub or GitLab |
| Structure | Left, upper (below Project) | Vertical | `Alt+7` | `⌘7` | Yes |
| Bookmarks | Left, upper | Vertical | `Alt+2` | `⌘2` | No, it is under More |
| Services | Left, lower (bottom area) | Horizontal | `Alt+8` | `⌘8` | Yes, when services exist |
| Run | Left, lower | Horizontal | `Alt+4` | `⌘4` | After the first run |
| Debug | Left, lower | Horizontal | `Alt+5` | `⌘5` | After the first debug session |
| Terminal | Left, lower | Horizontal | `Alt+F12` | `⌥F12` | Yes |
| Problems | Left, lower | Horizontal | `Alt+6` | `⌘6` | Yes |
| Git (Version Control) | Left, lower | Horizontal | `Alt+9` | `⌘9` | Yes, when VCS is enabled |
| Find | Left, lower | Horizontal | `Alt+3` | `⌘3` | After the first Find in Files |
| Build | Left, lower | Horizontal | — | — | After the first build |
| TODO | Left, lower | Horizontal | — | — | No, it is under More |
| Notifications | Right, upper | Vertical | — | — | Yes |
| Gradle | Right, upper | Vertical | — | — | Gradle projects only |
| Maven | Right, upper | Vertical | — | — | Maven projects only |
| Database | Right, upper | Vertical | — | — | Yes in the unified distribution (inferred) |
| AI Chat | Right, upper | Vertical | — | — | When AI is installed |
| Endpoints | Right or bottom (unknown) | Horizontal | — | — | Under More |
| Coverage | Right, upper | Vertical | — | — | After a coverage run |
| Documentation | Right, upper | Vertical | — | — | Created by pressing `Ctrl+Q` twice |
| Profiler | Bottom | Horizontal | — | — | Under More |

Notes:
- Terminal can also move into the editor area via the header context menu "Move to Editor".
- The Commit window can be set to float or window mode, or merged into the Git window's Log
  tab.

Impl notes: hold the defaults table in data (a `ToolWindowDefaults` registry contributed by
plugins), not in UI code. Each entry: `id`, `title`, `icon`, `anchor`, `group`, `order`,
`shortcut_win`, `shortcut_mac`, `availability_predicate`, `show_stripe_button_by_default`.

Sources:
- https://www.jetbrains.com/help/idea/tool-windows.html
- https://www.jetbrains.com/help/idea/viewing-modes.html
- https://www.jetbrains.com/help/idea/manipulating-the-tool-windows.html
- https://www.jetbrains.com/help/idea/tool-window-layouts.html
- https://www.jetbrains.com/help/idea/project-tool-window.html
- https://www.jetbrains.com/help/idea/new-ui.html
- https://www.jetbrains.com/help/idea/settings-appearance.html
- https://www.jetbrains.com/help/idea/commit-and-push-changes.html
- https://www.jetbrains.com/help/idea/terminal-emulator.html
- https://www.jetbrains.com/help/idea/reference-keymap-win-default.html
- https://www.jetbrains.com/help/idea/reference-keymap-mac-default.html
- https://plugins.jetbrains.com/docs/intellij/tool-window.html
- https://plugins.jetbrains.com/docs/intellij/toolbar.html

---

## 3. Editor area

### 3.1 Structure

Inside the editor island, top to bottom: tab row → breadcrumbs (if placed at the top) →
sticky lines (when scrolled) → text area with the gutter on the left and the scrollbar on the
right → breadcrumbs (if placed at the bottom). The inspection widget floats in the top-right
corner of the text area, at the top of the scrollbar.

### 3.2 Editor tabs

Settings: Editor | General | Editor Tabs. Defaults below are documented where a value is
given, otherwise (inferred).

| Setting | Values | Default |
|---|---|---|
| "Tab placement" | Top, Bottom, Left, Right, None | Top (inferred) |
| "Show tabs in" | "One row" + overflow ("Scroll the tabs panel", "Squeeze tabs"), or "Multiple rows" | One row, scroll (inferred) |
| "Show pinned tabs in a separate row" | bool | Off (inferred) |
| "Show file icon" | bool | On (inferred) |
| "Show file extension" | bool | On (inferred) |
| "Show directory for non-unique file names" | bool | On (inferred) |
| "Mark modified (*)" | bool | Off (inferred). Modified files are also shown by VCS file-status color on the tab title (inferred). |
| "Show full path on mouse hover" | bool | Off (inferred) |
| "Close button position" | Right, Left, None (inferred values) | Right (inferred) |
| "Sort tabs alphabetically" | bool | Off (inferred) |
| "Open new tabs at the end" | bool | Off (inferred). When off, a new tab opens to the right of the active tab. |
| "Enable preview tab" | bool | Off (inferred) |
| "Tab limit" | int | **10** (documented). A limit of 1 means a single editor with no tabs. |
| "When tabs exceed the limit" | "Close unchanged", "Close unused" | Close unused (inferred) |
| "When the current tab is closed, activate" | left tab / right tab / most recently opened (inferred values) | most recently opened (inferred) |
| Advanced: "Keep pinned tabs on the left" | bool | On (inferred) |

Tab anatomy (left to right): file-type icon, file name (with extension), optional
disambiguating directory, optional pin icon (pinned tabs), close `×`.

Behaviour:
- Close button: in the New UI, the `×` is shown on the active tab and appears on hover for
  inactive tabs (inferred). `Alt`+click on the `×` closes all other tabs. Middle-click
  closes a tab (inferred).
- Preview tab: the file name is in *italics*. Single-click in Project opens it there.
  Double-click the tab, or edit the file, to promote it to a regular tab.
- Pinned tabs: "Pin Tab" / "Unpin Tab" in the tab context menu. They show a pin icon and stay
  at the left. "Close All but Pinned" skips them.
- Double-click a tab: maximizes the editor (hides tool windows), the same as `Ctrl+Shift+F12`.
- Drag a tab out of the window to detach it into a new frame (`Shift+F4` also detaches).
- Islands: the selected tab has a filled rounded background plus an accent underline
  (theme-dependent). Inactive editor groups use the "inactive" variants (see 1.7).

Tab context menu (inferred set and order, using documented labels): "Close" (`Ctrl+F4`),
"Close Other Tabs", "Close All Tabs", "Close Tabs to the Left", "Close Tabs to the Right",
"Close All but Pinned", separator, "Pin Tab", separator, "Split Right", "Split Down",
"Split and Move Right", "Split and Move Down", "Move To Opposite Group", "Open In Opposite
Group", "Unsplit", "Unsplit All", separator, "Reopen Closed Tab", "Open in" ▸, "Copy Path/
Reference…", "Configure Editor Tabs…".

Shortcuts:

| Action | Win/Linux | macOS |
|---|---|---|
| Close tab | `Ctrl+F4` | `⌘W` (inferred) |
| Next / previous tab | `Alt+Right` / `Alt+Left` | `⌃→` / `⌃←` (inferred) |
| Switcher | `Ctrl+Tab` | `⌃⇥` |
| Detach tab | `Shift+F4` | `⇧F4` (inferred) |
| Maximize editor | `Ctrl+Shift+F12` | `⌘⇧F12` |
| Goto Next / Previous Splitter | `Alt+Tab` / `Alt+Shift+Tab` (inferred) | `⌥⇥` / `⌥⇧⇥` |
| Open in right split (from Project) | `Shift+Enter` | `⇧↩` |

### 3.3 Splits

- "Split Right" / "Split Down" create a new editor group showing the same file.
  "Split and Move Right/Down" move the file instead of duplicating it.
- Splits nest arbitrarily. Advanced Settings "Equalize proportions in nested splits".
- "Window | Editor Tabs" also has Stretch Editor to Top/Left/Bottom/Right.
- Exactly one editor group is current, and its selected tab uses the active colors.

### 3.4 Breadcrumbs

- Show the path from the file's root element to the element at the caret (e.g. class ›
  method). Each crumb is clickable and jumps there. Hover shows the element range (inferred).
- Settings: Editor | General | Breadcrumbs → "Show breadcrumbs", plus per-language
  checkboxes. Placement: Top or Bottom (also via right-click crumb → Appearance ›
  Breadcrumbs, or gutter context menu → Appearance).
- Default placement: **Top** (inferred; the docs do not state it). Many languages are on
  by default (Java, Kotlin, XML...).
- Separately, the status bar navigation bar (4.2) shows file-system path crumbs and the
  current code element.

### 3.5 Gutter

Left to right (inferred visual order for the New UI): line numbers (with breakpoints drawn
over them), gutter icons column (run, implement/override markers, bookmarks), VCS change
marker strip, folding arrows. Then comes the text.

| Feature | Detail | Default |
|---|---|---|
| Line numbers | Editor | General | Appearance → "Show line numbers": Absolute, Relative, Hybrid. Small font (editor size − 1). | On, Absolute (inferred mode) |
| Breakpoints over line numbers | A breakpoint replaces the line number with a red dot. Toggle: gutter → Appearance → "Breakpoints Over Line Numbers". | On |
| Breakpoint click | Click on an executable line toggles a line breakpoint. Click on a method declaration sets a method breakpoint, a class line breaks on construction, a field line sets a watchpoint. Drag moves it. | — |
| Run markers | A green triangle next to `main()` or tests. Click opens a popup: Run 'X', Debug 'X', Run with Coverage, Profile, Modify Run Configuration… | On |
| Implementation markers | Icons for implemented/overridden members. Click navigates or shows a chooser. | On |
| Bookmarks | `F11` anonymous, `Ctrl+F11` mnemonic (pick a digit or letter). | — |
| VCS change markers | Thin color bars: modified (light #C3D6E8 / dark #374752), added (#C9DEC1 / #384C38), whitespace-only (#EDDCBC / #4C4638), deleted = small triangle (#9F9F9F / #656E76). Click shows a popup with the old text plus Revert, Show Diff, Copy. | On ("Highlight modified lines in gutter") |
| Folding | Arrows appear on hover. "Show code folding arrows": Always / on hover / off. | On hover |
| Coverage | Green full, yellow partial, red uncovered. Click shows the statistics popup. | — |
| Gutter icons master switch | Editor | General | Gutter Icons → "Show gutter icons" + per-icon checkboxes. | On |

Gutter context menu → Appearance ▸: "Show Line Numbers", "Breakpoints Over Line Numbers",
"Show Indent Guides", "Show Sticky Lines", "Breadcrumbs" ▸; plus "Annotate with Git Blame",
"Add Bookmark", "Add Mnemonic Bookmark…".

Icon size in the gutter: 12×12 px per the icon guidelines (classic). New UI gutter icons
look like 14–16 px (inferred, verify).

### 3.6 Inspection widget (top-right of editor)

- Floats at the top-right corner of the text area, above the scrollbar.
- Shows severity icons with counts, e.g. red-circle "3", yellow-triangle "12", weak warning,
  typo, followed by up/down arrows to jump to the previous/next problem (`Shift+F2` / `F2`)
  (counts and arrows inferred from New UI behaviour).
- When the file has no problems, it shows a green check mark (inferred).
- Clicking a count opens the Problems tool window (documented).
- A hover popup shows a summary and a "Highlight:" level selector with "All Problems",
  "Syntax", "None" (inferred labels).
- When reader mode is active for a file, the widget shows a "Reader Mode" toggle
  (documented: "Error and warning highlighting, inspection widget" in reader-mode settings).
- While analysis runs, it shows a spinner or "Analyzing…" (inferred).

### 3.7 Scrollbar

- Error stripe: thin colored marks per problem along the vertical scrollbar, red for errors
  through blue for TODO. Hover shows a tooltip, and clicking navigates.
- Also marks search results, caret position, and VCS changes (inferred).
- "Show code lens on scrollbar hover": hovering the scrollbar shows a small floating preview
  of code at that position. Default on. Also offered in the scrollbar context menu.
- "Use contrast scrollbars" (Appearance → Accessibility).
- Scrollbars are overlay-style and thin, widening on hover (inferred).

### 3.8 Sticky lines

- On by default ("Show sticky lines while scrolling", Editor | General | Sticky Lines).
- Maximum **5** lines by default ("Maximum number of lines").
- Shows the headers of enclosing scopes (class, method, if/for...) pinned at the top of the
  text area while scrolling. Click a sticky line to jump to it (inferred).
- Context menu: "Configure Sticky Lines…", "Disable", "Disable for <Language>".
- Colors: Editor | Color Scheme | General | Editor | Sticky Lines (background, border,
  hovered).

### 3.9 Inlay hints and code vision

- Inlay hints: inline pseudo-text, for example parameter names before arguments. Font is one
  point smaller than code. Option "Use editor font for inlay hints" (default off, so the UI
  font is used; inferred).
- Settings: Editor | Inlay Hints, a tree of categories with checkboxes and a live preview,
  plus an exclusion list. Right-click a hint to disable that hint kind or its category.
  "Toggle Inlay Hints Globally" action.
- Code vision (block hints above declarations, e.g. "3 usages", "2 implementations", author):
  configured under Editor | Inlay Hints | Code vision (inferred). Position "Above the
  declaration" by default, with a "Right" option (inferred). Click on "N usages" opens the
  usages popup (inferred).
- Reader mode turns on code vision for usages, inheritors and related problems.

### 3.10 Reader mode

Settings (Editor | Reader Mode): "Enable Reader mode", then "Rendered documentation
comments" (toggle per comment via the gutter or `Ctrl+Alt+Q`), "Font ligatures", "Increased
line height", "Code vision hints for usages, inheritors, and related problems", "Error and
warning highlighting, inspection widget", "Format code according to preferred style".
Applies by default to read-only and library sources (inferred, see Open questions).

### 3.11 Floating code toolbar

- Appears when you hover a selection in Java, Kotlin, JSON, XML, YAML or SQL files.
- Default buttons in order: "View intention actions" (bulb), "AI actions" (only with AI),
  "Comment with line comment", "Reformat code".
- Controlled by Advanced Settings → Editor → "Hide floating toolbar for code editing". The docs
  describe disabling that option as how to enable it, which suggests it is **hidden by
  default** (see Open questions).
- Customizable under Menus and Toolbars → "Floating Code Toolbar".

### 3.12 Other editor details

- Minimap: none is built in. No minimap by default.
- Indent guides, whitespace, method separators: Editor | General | Appearance.
  Indent guides are on by default (inferred). Whitespace and method separators are off by
  default (inferred).
- Caret: Appearance settings "Caret blinking (ms)", "Use smooth caret blinking", "Use block
  caret", "Use full line height caret", "Use smooth caret movement" (Snappy / Gliding).
  Smooth caret animation is new in 2026.1.
- Zoom indicator: a small overlay at the bottom of the editor while the font is zoomed. On by
  default (Advanced: "Show zoom indicator"). Global font size: `Alt+Shift+.` / `Alt+Shift+,`.
  "Change font size with Ctrl+Mouse Wheel in" (Editor | General) is off by default
  (inferred).
- Intention bulb: see 6.10.
- Show Running List (Run menu) shows active sessions in the editor's top-right corner.

### 3.13 Empty editor placeholder

When no file is open, the editor island shows centered, muted text lines, one action per
line, each with its shortcut (labels and order **inferred**, verify):

| Line | Win/Linux | macOS |
|---|---|---|
| Search Everywhere | Double Shift | Double ⇧ |
| Go to File | `Ctrl+Shift+N` | `⇧⌘O` |
| Recent Files | `Ctrl+E` | `⌘E` |
| Navigation Bar | `Alt+Home` | `⌥↖` (inferred) |
| Drop files here to open them | — | — |

The placeholder text should come from the keymap so it updates live (inferred design).

Sources:
- https://www.jetbrains.com/help/idea/using-code-editor.html
- https://www.jetbrains.com/help/idea/editor-basics.html
- https://www.jetbrains.com/help/idea/settings-editor-tabs.html
- https://www.jetbrains.com/help/idea/editor-gutter.html
- https://www.jetbrains.com/help/idea/settings-gutter-icons.html
- https://www.jetbrains.com/help/idea/sticky-lines.html
- https://www.jetbrains.com/help/idea/reader-mode.html
- https://www.jetbrains.com/help/idea/inlay-hints.html
- https://www.jetbrains.com/help/idea/settings-editor-appearance.html
- https://www.jetbrains.com/help/idea/settings-editor-general.html
- https://www.jetbrains.com/help/idea/working-with-source-code.html
- https://www.jetbrains.com/help/idea/new-ui.html
- https://www.jetbrains.com/help/idea/creating-and-running-your-first-java-application.html
- https://www.jetbrains.com/idea/whatsnew/2026-1/
- https://plugins.jetbrains.com/docs/intellij/icons-style.html
- https://plugins.jetbrains.com/docs/intellij/typography.html

---

## 4. Status bar

### 4.1 Layout

```
[ navigation bar crumbs: project › src › main › java › pkg › Foo › bar() ]   ...   [progress] [widgets ...] [notifications?]
```

- Left part: the **navigation bar**. If the navigation bar is moved to the top or hidden,
  the left part instead shows the latest event message. Clicking the message opens the
  Notifications tool window, and right-click offers "Copy".
- Right part: background progress, then the widgets.
- Show/hide the whole bar: View | Appearance | Status Bar. Right-click the bar to get a
  checkbox list of widgets.

### 4.2 Navigation bar (New UI)

- Default location: **in the status bar** (bottom). View | Appearance | Navigation Bar → "Top",
  "In Status Bar" (inferred label), "Don't show".
- Focus: `Alt+Home` (Win/Linux), `⌥↖` macOS (inferred). Focus highlights the last crumb.
  Left/right arrows move between crumbs. `Down` or `Enter` opens a popup listing the children of
  that crumb (with speed search). Escape returns to the editor (inferred).
- Crumbs: project › directories › file › class › member (members are shown when the
  file has structure; inferred). Each crumb has an icon and a name, separated by a chevron.
- Clicking a crumb opens the same popup listing its children.
- When the path is too long, leading crumbs collapse (inferred).

### 4.3 Widgets (right side, documented left-to-right order)

| # | Widget | Example | Default visible | Click |
|---|---|---|---|---|
| 0 | Background tasks / progress | small progress bar + task name | While a task runs | Opens the Processes manager popup with per-task cancel buttons (inferred) |
| 1 | Caret position | `52:11` (with a selection: `52:11 (24 chars, 1 line break)`) | Yes | Opens "Go to Line:Column" (`Ctrl+G`) |
| 2 | Line separator | `LF` / `CRLF` / `CR` | Yes | Popup to change the file's line separators |
| 3 | File encoding | `UTF-8` | Yes | Popup with "Reload in…" / "Convert to…" encoding choices (inferred) |
| 4 | Column selection | `Column` | Only when column mode is on (`Alt+Shift+Insert`) | Toggles off |
| 5 | Read-only toggle | lock / unlock icon | Yes, when relevant (inferred) | Toggles the file's writability |
| 6 | Indent | `4 spaces` / `Tab` | Yes | Popup: configure indents for the language, disable indent detection |
| 7 | IDE error indicator | red icon | Only after an internal error | Opens the IDE errors dialog with "Report"/"Clear" |
| — | Memory indicator | `512 of 2048M` | **Off** (inferred: off by default since New UI) | Click runs GC (inferred) |
| — | VCS branch | branch name | Off; branch moved to header | — |
| — | Others (inferred): code-style/EditorConfig, "Power Save Mode", AI quota, inspections, JSON schema, git line status... | | Varies | |

Behaviour rules:
- A widget whose value does not apply (e.g. no editor focused) hides automatically.
- Widgets use 12×12 px icons per the icon guidelines (classic value; may be 16 px in the New
  UI, inferred).
- Hover: rounded background highlight. Tooltip shows the full meaning (e.g. "Line Separator:
  LF") (inferred).

Sources:
- https://www.jetbrains.com/help/idea/status-bar.html
- https://www.jetbrains.com/help/idea/guided-tour-around-the-user-interface.html
- https://www.jetbrains.com/help/idea/new-ui.html
- https://www.jetbrains.com/help/idea/notifications.html
- https://plugins.jetbrains.com/docs/intellij/icons-style.html

---

## 5. Welcome screen and New Project wizard

### 5.1 Welcome screen

Shown on first launch (after the optional Import Settings step) and whenever all projects are
closed. A fixed-size frame with a left navigation column and a content area (left-nav
items documented, order inferred):

| Left nav item | Content |
|---|---|
| "Projects" | Top: search field ("Search projects"), buttons "New Project", "Open", "Clone Repository" (the 2026 help calls the action "Get a project from a version control system"; button caption inferred). Below: the recent projects list. Each row has a colored square with initials (project icon), name, and path. Rows support grouping into folders (inferred), a ⋮ menu with "Open in New Window" / "Show in Explorer|Finder" / "Remove from Recent Projects" / "Move to Group" (inferred). When the list is empty, the page shows large cards for "New Project", "Open", "Clone Repository". |
| "Remote Development" | Entry points for SSH, Dev Containers, and other remote backends. |
| "Customize" | Color theme combo + "Sync with OS" checkbox, color scheme, Accessibility (IDE font size, red-green vision adjustment), Keymap combo, "Import Settings…" link, "All settings…" link (opens the default-settings dialog). |
| "Plugins" | Marketplace / Installed tabs, the same as Settings | Plugins. |
| "Learn" | Learning courses (in-IDE onboarding project) and links to docs, videos. |

- Bottom-left: a gear/settings button and a help link (inferred).
- New in 2026.1 (Ultimate): a dedicated Kubernetes welcome screen entry.
- First run flow: Import Settings dialog (choose a source IDE, including VS Code. "Other
  Options" lists IDEs not used for 6+ months. Button "Import Settings", or skip).

### 5.2 New Project wizard

Opened with "New Project" on the welcome screen or "File | New | Project…". A modal dialog with
a list of generators on the left and a form on the right.

Left list (inferred order, consistent with the 2025–2026 wizard):
1. "New Project" (language-based: Java, Kotlin, Groovy, HTML, JavaScript... with "More via
   plugins…")
2. "Empty Project"
3. Separator + "Generators" header: "Maven Archetype", "Jakarta EE", "Spring Boot", "JavaFX",
   "Quarkus", "Micronaut", "Ktor", "Kotlin Multiplatform", "Compose for Desktop", "HTML",
   "React", "Express", "Angular CLI", "Vue.js", "Vite", "Next.js", "Cypress"... (the docs
   list these generator sections but do not give the list order)
4. "More via plugins…" link at the bottom.

New Project → Java form, top to bottom (labels documented, defaults partly inferred):

| Field | Control | Default |
|---|---|---|
| "Name" | text | `untitled` (inferred) |
| "Location" | path field with browse | `~/IdeaProjects` (inferred); a hint below shows the resulting path "Project will be created in: …" (inferred) |
| "Create Git repository" | checkbox | Off (inferred) |
| "Language" | segmented buttons: Java, Kotlin, Groovy, + more | Java |
| "Build system" | segmented buttons: "IntelliJ", "Maven", "Gradle" | Gradle in recent releases (inferred, verify) |
| "JDK" | combo with detected JDKs, "Add JDK from Disk…", "Download JDK…" | Most recent detected JDK (inferred) |
| "Gradle DSL" (docs: build script language) | "Kotlin" / "Groovy" | Kotlin (inferred) |
| "Add sample code" | checkbox | On (inferred); creates a HelloWorld-style `Main` class |
| "Generate code with onboarding tips" | checkbox (indented under sample code) | On (inferred) |
| "Advanced Settings" | collapsible section, collapsed | — |
| ↳ "Gradle distribution" | "Wrapper" / "Local installation" | Wrapper (inferred) |
| ↳ "Gradle version" | combo | latest supported (inferred) |
| ↳ "Auto-select" / "Use these settings for future projects" | checkbox | (inferred) |
| ↳ "GroupId" | text | `org.example` (inferred) |
| ↳ "ArtifactId" | text | same as Name (inferred; documented that ArtifactId is the project name for Maven) |

Buttons at the bottom right: "Create" and "Cancel" (single-page wizard; "Next" appears only
for generators with a second page, e.g. Spring Boot dependencies). Help `?` bottom-left
(inferred).

- "Empty Project" always creates `.gitignore`.
- Validation is shown inline under the field with a red outline (see 8.3).

Sources:
- https://www.jetbrains.com/help/idea/run-for-the-first-time.html
- https://www.jetbrains.com/help/idea/new-project-wizard.html
- https://www.jetbrains.com/help/idea/creating-and-running-your-first-java-application.html
- https://www.jetbrains.com/idea/whatsnew/2026-1/

---

## 6. Popups and dialogs

General popup rules (UI guidelines): popups are semi-modal, have no close button, and close
on focus loss and on `Escape`. They may have a title, may be movable and resizable, may
remember their size, and may nest (choosing an item opens a sub-popup). Action lists support
number keys for fixed lists, speed search (type to filter) for large lists, and mnemonics.

### 6.1 Search Everywhere

- Open: double `Shift` (both keymaps), or Navigate | Search Everywhere. Opens on the **All**
  tab and shows recent files when the query is empty.
- Tab shortcuts:

| Tab | Win/Linux | macOS |
|---|---|---|
| All | double `Shift` | double `⇧` |
| Classes | `Ctrl+N` | `⌘O` (inferred) |
| Files | `Ctrl+Shift+N` | `⇧⌘O` |
| Symbols | `Ctrl+Alt+Shift+N` | `⌥⌘O` |
| Actions | `Ctrl+Shift+A` | `⇧⌘A` |
| Text | none by default (inferred) | none (inferred) |

- Tab row order (inferred): All, Classes, Files, Symbols, Actions, Endpoints (when a URL
  framework is present), Text. "Git" is optional (Advanced Settings).
- `Tab` / `Shift+Tab` switch tabs. Pressing a tab's shortcut while the popup is open
  switches to that tab (inferred).
- Header right side: "Include non-project items" checkbox (`Alt+N`; pressing double `Shift`
  again toggles it). On All, this is the "All Places" scope. There is also a **Filter** icon
  (e.g. exclude file types or languages), a **Preview** toggle that shows a code preview pane
  under the list (inferred: added in 2024), and **"Open in Find Tool Window"** (disabled on
  Actions).
- List: rows grouped by category with headers on All. Each group ends with a "more…" row.
  `Ctrl+Down` / `Ctrl+Up` jump between groups (documented as jumping to "more" and back
  to the top).
- Text tab offers match case, words, regex toggles. Text results also show at the bottom of
  All when other results are few ("Show text search results in Search Everywhere", Advanced).
- Special queries: `/` lists settings groups (`/plugins`, `/appearance`, `/system`,
  `/inspections`, `/registry`, `/intentions`, `/templates`, `/vcs`). `/` + URL fragment
  searches endpoints. Arithmetic is evaluated inline (calculator: `+ - * / ^ sqrt sin cos tan`,
  hex/bin/oct). On the Actions tab, `Alt+Enter` on an action assigns it a shortcut. Toggle
  settings show an ON/OFF switch in the row.
- Size: a wide popup centered in the upper part of the frame, resizable, remembers its size
  (inferred).

### 6.2 Recent Files (`Ctrl+E` / `⌘E`)

- Two-pane popup (inferred from screenshots): left column lists tool windows (with
  shortcuts), right column lists recently opened files. A title row "Recent Files" plus a
  "Show edited only" checkbox (pressing `Ctrl+E` again toggles it).
- Type to filter. Arrow keys, Enter opens, `Delete` removes the entry (inferred).
- Recent Changes: `Alt+Shift+C` / `⌥⇧C`.

### 6.3 Recent Locations (`Ctrl+Shift+E` / `⇧⌘E` inferred)

- Vertical list of code snippets (a few lines each) with file name and breadcrumbs above each
  snippet. The latest location is first.
- Checkbox "Show changed only" (help text also calls it "Show edited only").
  `Ctrl+Shift+E` again toggles it.
- Type to filter by code, file name or breadcrumb. `Delete`/`Backspace` removes a location
  (and drops it from back/forward history).

### 6.4 Switcher (`Ctrl+Tab` / `⌃⇥`)

- Opens while `Ctrl` is held. Left column: tool windows. Right column: open files in
  most-recent order (inferred layout, similar to Recent Files).
- `Tab`/`Ctrl+Tab` moves down, `Ctrl+Shift+Tab` moves up, arrow keys work while `Ctrl` is
  held, and releasing `Ctrl` opens the selection. The popup stays open if released quickly
  without moving (inferred).
- On macOS, `⌃`+arrows may be taken by Mission Control.

### 6.5 Settings dialog

- Open: `Ctrl+Alt+S` (Win/Linux, File | Settings), `⌘,` (macOS, IntelliJ IDEA | Settings),
  or the header gear → "Settings…".
- **Non-modal by default** (Advanced Settings "Show settings in a non-modal dialog"). It is
  bound to its project, so switching projects requires reopening it. A "Pin Window" button
  keeps it above the project frame.
- Layout:
  - Top-left: search field with a magnifier. Typing filters the tree and highlights matching
    option labels in the right pane (inferred).
  - Left: category tree. Top-level nodes in order (inferred): Appearance & Behavior,
    Keymap, Editor, Plugins, Version Control, Build, Execution, Deployment, Languages &
    Frameworks, Tools, Settings Sync (inferred), Advanced Settings. Nodes for project-level
    pages carry the "For current project" icon (a small document/project glyph after the
    label).
  - Right: page header with breadcrumbs ("Appearance & Behavior › Appearance") and, on the
    right, a "Reset" link when the page is modified, plus back/forward arrows for page
    history (both inferred).
  - Page body: form.
  - Bottom bar: `?` help button (left), then "OK", "Cancel", "Apply" (right). "Apply" is
    enabled only when something is modified (inferred).
- Modified pages show their tree label in a highlight color (inferred).
- Restore defaults: File | Manage IDE Settings | Restore Default Settings → "Restore and
  Restart".

### 6.6 Project Structure dialog

- Open: `Ctrl+Alt+Shift+S` (File | Project Structure, or gear → Project Structure). macOS
  `⌘;` (inferred).
- Three-pane layout (inferred): left category list; middle element list (modules, libraries)
  with +/−/copy toolbar; right detail editor.
- Left categories (inferred): "Project Settings": Project, Modules, Libraries, Facets,
  Artifacts. "Platform Settings": SDKs, Global Libraries. Then "Problems" (with a count).
- Project page fields (documented names): Name, SDK, Language level, Compiler output.
- Buttons: OK, Cancel, Apply.

### 6.7 Run/Debug Configurations dialog

- Open: run widget → "Edit Configurations…", Run | Edit Configurations, or `Alt+Shift+F10` → `0`.
- Left pane: tree of configurations grouped by type, toolbar with "+" (`Alt+Insert`, opens a
  type chooser such as "Application", "JUnit", "Gradle"...), "−", copy (inferred), "Save",
  "Move into New Folder", sort (inferred). Link at the bottom: "Edit configuration
  templates…".
- Right pane: "Name" field, "Store as project file" checkbox (top-right), type-specific
  form, "Modify options" link that adds optional fields, and a "Before launch" task list
  (inferred).
- Footer: "Run" (or "Debug"), "OK", "Cancel", "Apply".

### 6.8 Commit (tool window vs modal)

- Default: **non-modal commit in the Commit tool window** (left side, `Alt+0`/`⌘0`). `Ctrl+K`
  (Commit…) opens or focuses it. A modal commit dialog exists only through the bundled
  "Modal Commit Interface" plugin, enabled under Settings | Advanced Settings | Version Control.
- Anatomy (top to bottom): toolbar (refresh, rollback, diff, group by, expand/collapse;
  inferred) → changes tree with changelists and an "Unversioned Files" node, each with
  checkboxes → commit message editor (with a message-history button) → "Amend" checkbox
  (chevron to choose a commit) → buttons "Commit" and "Commit and Push…" (`Ctrl+Alt+K`), plus
  a gear opening the commit options (`Ctrl+O`).
- Commit options: Author, "Sign-off commit"; Commit Checks: "Reformat code", "Rearrange
  code", "Optimize imports", "Cleanup", "Update copyright", "Check malicious dependencies";
  Advanced: "Run advanced checks after a commit is done", "Analyze code", "Check TODO",
  "Run Configuration"; After Commit: "Upload files to".

### 6.9 Notifications

- Balloons: bottom-right of the frame by default (a "Notification location" setting exists).
  A balloon has an icon (info/warning/error), title, body, inline action links, a ⋯ menu and
  a close × on hover (anatomy inferred). Normal balloons fade after a short time (exact
  timeout undocumented), and sticky ones stay.
- Per-group display ("Popup type"): "Balloon", "Tool window balloon", "Sticky balloon", "No
  popup", plus checkboxes "Show in tool window", "Play sound", "Read aloud" (macOS).
  Global "Display balloon notifications".
- Notifications tool window (right stripe, inferred): sections "Suggestions" and
  "Timeline". Unread items are marked, and stripe badge dots are blue for regular and red for
  errors or important items. Suggestions have ⋯ → "Don't show again" / "Don't show again for
  this project". Entries persist until cleared or the project reopens. Header actions
  include "Mark all as read" and "Clear All" (inferred).

### 6.10 Intention actions popup (`Alt+Enter` / `⌥↩`)

- A bulb sits in the gutter at the caret line (inferred column: the gutter-icons area): a
  **yellow bulb** means intentions are available, and a **red bulb with "!"** means a
  quick-fix is available. "Show intention bulb" (Editor | General | Appearance) hides both.
- Popup: a list of context actions (quick-fixes first, then intentions; inferred ordering),
  each with an icon. To the right, a **preview pane** shows the resulting code diff when the
  intention supports it, otherwise a description. Toggle with `Ctrl+Q`, or the setting "Show
  preview for intention actions when available".
- Each row has a right-arrow submenu: "Disable '<name>'", plus for inspection fixes
  "Edit inspection profile setting", "Suppress for statement/method/class", "Fix all
  '<inspection>' problems in file" (inferred), and a ⋮ icon with "Assign shortcut…".
- Speed search by typing, and number keys work (inferred).

### 6.11 Completion, Quick Documentation, Parameter Info

Completion popup (`Ctrl+Space` / `⌃Space`):
- Anchored below the caret, aligned with the start of the identifier. Each row: kind icon
  (class/method/field/keyword/live template), name with the typed prefix bold or
  highlighted, parameters or tail text in grey, type text right-aligned in grey (anatomy
  inferred, standard).
- A footer/bottom bar with a ⋮ menu (contains "Sort by Name") and a hint such as "Press ⏎ to
  insert, ⇥ to replace" (inferred).
- `Enter` inserts. `Tab` replaces the identifier to the right of the caret.
- A documentation side popup appears after a delay if "Show the documentation popup in N ms"
  is on.
- Since 2025.3, typing `.` mixes code, postfix and command completion, and `..` shows only
  actions with a preview.

Quick Documentation (`Ctrl+Q` / `F1`):
- Shown on hover by default ("Show quick documentation on hover"; delay = "Tooltip delay").
- Popup ⋮ menu: font size adjust, "Show Toolbar", "Edit Source"/navigate to source,
  "Show on Mouse Move".
- Pressing `Ctrl+Q` again moves it into the Documentation tool window, where the `*` tab is
  the unpinned auto-updating tab ("Keep This Documentation" pins it). `Shift+F1` opens external
  docs.

Parameter Info (`Ctrl+P` / `⌘P`): appears automatically about 1000 ms after `(`. It shows
all overloads, one per row, with the current parameter in bold (inferred) and inapplicable
overloads greyed (inferred). Option "Show full method signatures".

Type Info `Ctrl+Shift+P`. Context info `Alt+Q`.

### 6.12 Branches popup (VCS widget)

- Top: search field + Fetch icon + gear ("Group by Directory", "Show Recent Branches",
  "Show Tags").
- Action rows (inferred, standard New UI order): "Update Project…", "Commit…", "Push…",
  "New Branch…", "Checkout Tag or Revision…", separator.
- Groups: Favorites (main is a favorite by default), "Recent" (up to 5), "Local", "Remote",
  "Tags". Branches with `/` are grouped as folders.
- Branch submenu: "Checkout", "New Branch from '<b>'…", "Checkout and Update", "Compare with
  Current", "Show Diff with Working Tree", "Rebase…", "Merge…", "Rename…", "Delete"
  (merge/rebase inferred).
- Keys: `Space` toggles favorite, `Ctrl+F` focuses search, `Ctrl+C` copies branch name.

### 6.13 Find in Files (`Ctrl+Shift+F` / `⇧⌘F`)

- Non-modal, resizable popup with a pin (pin inferred).
- Top row: title "Find in Files", match counter (inferred), a "Filter" icon (e.g. "Anywhere",
  "In Comments", "In String Literals", "Except Comments"...; labels inferred), and an
  option toggle area.
- Search field with history (`Alt+Down`), multi-line toggle, and toggles **Match case**
  (`Aa`), **Words** (`W`), **Regex** (`.*`).
- File mask checkbox + combo (`*`, `?`, `,`, `!` syntax).
- Scope row: "In Project", "Module", "Directory", "Scope" (segmented buttons; "In Project"
  inferred), with a combo/field for the chosen module, directory or scope.
- Results list (line text, file name, line number on the right), then a **preview editor**
  pane below it.
- Bottom: "Open Results in New Tab" checkbox, "Open in Find Window" button (`Ctrl+Enter`
  inferred). Replace in Files (`Ctrl+Shift+R` / `⇧⌘R`) adds a replace field and "Replace" /
  "Replace All" buttons.

Sources:
- https://plugins.jetbrains.com/docs/intellij/popups.html
- https://www.jetbrains.com/help/idea/searching-everywhere.html
- https://www.jetbrains.com/help/idea/recent-files-and-changes.html
- https://www.jetbrains.com/help/idea/navigating-through-the-source-code.html
- https://www.jetbrains.com/help/idea/settings-preferences-dialog.html
- https://www.jetbrains.com/help/idea/configuring-project-and-ide-settings.html
- https://www.jetbrains.com/help/idea/guided-tour-around-the-user-interface.html
- https://www.jetbrains.com/help/idea/project-structure-dialog.html
- https://www.jetbrains.com/help/idea/run-debug-configuration.html
- https://www.jetbrains.com/help/idea/commit-and-push-changes.html
- https://www.jetbrains.com/idea/whatsnew/2025-1/
- https://www.jetbrains.com/help/idea/notifications.html
- https://www.jetbrains.com/help/idea/intention-actions.html
- https://www.jetbrains.com/help/idea/auto-completing-code.html
- https://www.jetbrains.com/help/idea/viewing-reference-information.html
- https://www.jetbrains.com/help/idea/manage-branches.html
- https://www.jetbrains.com/help/idea/finding-and-replacing-text-in-project.html
- https://www.jetbrains.com/idea/whatsnew/2025-3/

---

## 7. Theming, fonts, density, scaling

### 7.1 UI themes (bundled in 2026.2)

| Theme | Family | Notes |
|---|---|---|
| "Islands Dark" | Islands | Default look since 2025.3 (blog/whatsnew). |
| "Islands Light" | Islands | |
| "Islands Darcula" | Islands | Islands geometry with Darcula colors. |
| "Dark" | New UI classic (non-island) | The help page says the IDE uses Dark unless changed on first run (conflicts with the Islands default; see Open questions). |
| "Light" | New UI classic | |
| "Light with Light Header" | New UI classic | Light theme with a light header (the default Light has a dark header). |
| "Darcula" | Legacy look | |
| "High Contrast" | Accessibility | |

- "Sync with OS" checkbox next to the Theme combo, with a gear to choose which dark and light
  theme pair to use. Default pair (inferred): Islands Dark / Islands Light.
- Default per OS: not documented; we assume Islands Dark on all OSes with "Sync with OS"
  off (inferred). Verify on a clean install on each OS.
- "Get More Themes" link to the Marketplace. Quick switch: `Ctrl+\`` → Theme.

### 7.2 Editor color schemes

Documented bundled schemes: "Dark" (for Dark), "Light" (for Light and Light with Light
Header), "High Contrast", "Classic Light" (old macOS/Windows 10 Light look), "Darcula",
"Darcula Contrast". Each theme selects its matching scheme by default. Islands Dark is
assumed to use "Dark" and Islands Light "Light" (inferred). The default scheme for the
current theme is marked in the Scheme list.

### 7.3 Fonts

| Item | Value | Source status |
|---|---|---|
| UI font | **Inter**, 13 px on all OSes | Documented (typography guideline, new-ui page) |
| UI type scale | H1 18 px bold, H2 16 px bold, Default 13 px regular, Default semibold 13, Paragraph 13 with 16 px line height, Medium 12 regular and semibold | Documented |
| Editor font | **JetBrains Mono** | Documented (typography guideline) |
| Editor size | **13** (pt as labelled) | Inferred: the documented default isn't stated on the font help page |
| Editor line height | **1.2** | Documented |
| Ligatures | Off by default (inferred) | |
| Gutter line numbers | editor size − 1 | Documented (typography) |
| Inlay hints | editor size − 1, UI font | Documented |
| "Use custom font" (UI) | Off by default (inferred) | |

### 7.4 Density, zoom, accessibility

- Compact mode: see 1.5.
- "Zoom IDE" (View | Appearance, or Appearance → Accessibility → "Zoom"): 70%–200% steps
  (popup lists values), 100% default. "Reset to default" appears when not 100%. Zoom scales
  everything uniformly, with no re-layout rules beyond scaling.
- OS HiDPI scaling is honoured on top of Zoom (inferred).
- Accessibility: "Support screen readers", "Use contrast scrollbars", "Adjust colors for
  red-green vision deficiency", "Use simplified splash screen".
- UI Options: "Enable mnemonics in controls", "Enable mnemonics in menu", "Display icons in
  menu items", "Keep popups open for toggle items", "Smooth scrolling" (on by default),
  "Background Image…".
- Tree views: "Show indent guides", "Use smaller indents".
- Antialiasing: "IDE" and "Editor" each Subpixel / Greyscale / No antialiasing. Defaults
  (inferred): Subpixel on Windows, Greyscale on macOS and Linux.

### 7.5 Icons

- New UI icons: monochrome outline set with distinct colors for semantic accents. Sizes:
  16×16 for toolbars and trees (14×14 live area in a 1 px padding), 20×20 for stripes in
  normal mode and 16×16 in compact mode, 12×12 for gutter and status bar (classic guideline
  value), 32×32 in dialogs. Stroke 2 px (classic guideline; the New UI set uses a thinner
  ~1 px stroke, inferred).
- archipel-ide must draw its own icon set (clean-room). Copy only sizes and semantics.

Sources:
- https://www.jetbrains.com/help/idea/user-interface-themes.html
- https://www.jetbrains.com/help/idea/settings-appearance.html
- https://www.jetbrains.com/help/idea/configuring-colors-and-fonts.html
- https://www.jetbrains.com/help/idea/settings-editor-font.html
- https://www.jetbrains.com/help/idea/ide-zoom-level.html
- https://www.jetbrains.com/help/idea/new-ui.html
- https://plugins.jetbrains.com/docs/intellij/typography.html
- https://plugins.jetbrains.com/docs/intellij/icons-style.html
- https://plugins.jetbrains.com/docs/intellij/supporting-islands-theme.html
- https://blog.jetbrains.com/platform/2025/12/meet-the-islands-theme-the-new-default-look-for-jetbrains-ides/

---

## 8. Cross-cutting interaction rules

### 8.1 Focus model

- Exactly one focus owner: the editor, a tool window, the header, or the status-bar
  navigation bar.
- `Escape` from a tool window returns to the editor. `Escape` in the editor with nothing
  to cancel does nothing (inferred).
- `F12` returns to the last tool window. Tool window shortcuts focus or toggle.
- Speed search: in any tree or list, typing starts a speed search with an inline field
  over the component (inferred position: top-left overlay) and highlights matches.
  `Ctrl+F` in tool windows starts it explicitly.
- Unfocused islands keep their selection drawn with the inactive selection color (inferred).

### 8.2 Context menus

- Every list, tree, editor, tab and stripe has a context menu. Items show their shortcut
  right-aligned. Submenus open on hover after a short delay (inferred). Disabled items are
  greyed. Separators group related items.
- Toggle items show a check mark. "Keep popups open for toggle items" keeps the menu open
  after toggling.
- Notable popup menus: Generate (`Alt+Insert`), Refactor This (`Ctrl+Alt+Shift+T`), New
  (`Alt+Insert` in Project), VCS Operations (`Alt+\``), Quick Switch Scheme (`Ctrl+\``).

### 8.3 Validation (dialogs and forms)

- Errors are shown inline next to the offending field: the field gets a red outline and a
  tooltip-like message appears under or beside it (inferred from the UI guidelines topic
  list; that page was not fetched). The OK/Create button is disabled while errors exist, or
  clicking it focuses the first invalid field (inferred).

### 8.4 Toolbar rules (all toolbars)

- Horizontal toolbars are left-aligned, with rarely used items right-aligned. Order: actions,
  view options, preview toggle, settings.
- Separators only between logical groups. Toolbars with 5 or fewer icons can omit them.
- Overflow goes into a trailing chevron (`»`) whose hover reveals the hidden items.
- Never stack two toolbars. Use dropdown buttons instead.
- Buttons are 16 px icons on a ~24–28 px square hit area, with a hover fill and a pressed fill
  (sizes inferred).

### 8.5 Persistence

| State | Scope (inferred) |
|---|---|
| Tool window layout, sizes, visibility, view modes | Per project (workspace) |
| Named layouts | Application |
| Open editors, splits, pinned tabs, caret positions | Per project |
| Header customization, theme, fonts, zoom, compact mode, main-menu mode | Application |
| Recent projects list, groups | Application |
| Popup sizes (Search Everywhere, Find in Files) | Application |
| Settings dialog last selected page | Application |

Impl notes: map application scope to `archipel-ide-core` settings and project scope to the
project workspace file. UI only reads and writes through typed commands (AGENTS.md).

Sources:
- https://plugins.jetbrains.com/docs/intellij/toolbar.html
- https://plugins.jetbrains.com/docs/intellij/popups.html
- https://plugins.jetbrains.com/docs/intellij/layout.html
- https://www.jetbrains.com/help/idea/guided-tour-around-the-user-interface.html
- https://www.jetbrains.com/help/idea/settings-appearance.html
- https://www.jetbrains.com/help/idea/tool-window-layouts.html

---

## 9. Open questions (verify in a real IntelliJ IDEA 2026.2)

Defaults and pixels:
1. **Default theme on first launch**: the 2025.3 release notes and the platform blog say Islands
   is the default, but the 2026.2 help page says the IDE uses "Dark" unless changed on first run.
   Is "Dark" just the name shown, or the non-island theme? Check a clean install on all three
   OSes, with and without "Sync with OS".
2. Default "Main menu" mode on Windows/Linux: is it "Hide under Hamburger Button" or (since
   2025.1) "Merge with Main Toolbar"?
3. Header, stripe, tool-window-header, editor-tab and status bar heights (normal/compact).
   The 40/32/30 px values here are guesses.
4. Island radius and gap in compact mode (8 px / 3 px by the halving and minus-one rules).
   Confirm the rendering rules on HiDPI.
5. Exact position and contents of AI widgets in the header (AI chat icon, quota, agent selector)
   in 2026.2, and the default anchor of the AI Chat tool window.
6. Content and order of the right part of the header: is there a separate Search
   Everywhere icon by default? Is "Code With Me" or a "Notifications" bell there?
7. Run widget colors while running or debugging, and how several running configurations are shown.
8. Gear (IDE and Project Settings) menu: exact item list and order.

Tool windows:
9. Default stripe placement and order for every tool window in table 2.7 (the anchors are
   inferred). In particular, Structure (left upper vs left lower), Notifications (right),
   Database, Endpoints, Build, Services.
10. The exact labels of "Move to" targets and where the "More tool windows" button sits.
11. Click semantics on a stripe button for a visible but unfocused window: focus or hide?

Editor:
12. Editor tab defaults (close button policy on inactive tabs, Mark modified, preview tab default,
    "When tabs exceed the limit" default, "activate on close" default).
13. Breadcrumbs default placement (top vs bottom) in 2026.2. Does the status-bar navigation
    bar duplicate code-element crumbs?
14. Is the floating code toolbar hidden by default? The help page implies yes.
15. Inspection widget: exact contents, hover popup labels, and the reader-mode toggle text.
16. Empty-editor placeholder lines and order.
17. Code vision defaults (usages, implementations, inheritors, author) and default position.
18. Reader mode default scope (library/read-only files only?).
19. Editor font default size (13?) and ligature default. Does 2026.2 ship "JetBrains Mono" with
    ligatures off?
20. Line-number mode default (Absolute assumed).

Status bar:
21. Default widget set in 2026.2 and its exact order, including the memory indicator default
    (assumed off) and any AI/quota widget.
22. Exact label of the status-bar option in View | Appearance | Navigation Bar ("In Status Bar"?).

Popups/dialogs:
23. Search Everywhere tab order and whether a Preview pane exists and is on by default.
24. Settings dialog: presence of "Reset" link, breadcrumbs, back/forward arrows. Top-level tree
    order (where does "Settings Sync" or "Backup and Sync" sit?).
25. Project Structure macOS shortcut (`⌘;`) and category list in 2026.2.
26. Welcome screen: exact button captions ("Clone Repository" vs "Get from VCS"), nav order,
    and whether 2026.x moved to a non-modal, tab-based welcome inside a project frame.
27. New Project wizard: default build system (Gradle vs IntelliJ), "Add sample code" default,
    the onboarding-tips checkbox, Advanced Settings fields, and generator list order.
28. Notification balloon timeout, maximum stacked balloons, and the Notifications tool window's
    header actions.
29. Find in Files: exact filter labels and scope button captions, and whether a pin exists.

Sources disagreeing:
- Default theme (Islands vs "Dark"): whatsnew 2025.3 + platform blog vs help page
  user-interface-themes.html.
- Recent Locations checkbox label: "Show edited only" (text) vs "Show changed only"
  (screenshot caption) on recent-files-and-changes.html.
- Stripe icon size: tool-window guideline says 20×20/16×16, icons-style guideline lists
  13×13 for tool windows (classic UI value).
- 2026.2 "What's New" page did not render via fetch. UI changes specific to 2026.2 (beyond
  AI agents, Copilot, logpoints) could not be confirmed and must be checked directly.

Sources:
- https://www.jetbrains.com/help/idea/user-interface-themes.html
- https://www.jetbrains.com/idea/whatsnew/2025-3/
- https://www.jetbrains.com/idea/whatsnew/2026-2/
- https://blog.jetbrains.com/idea/2026/07/intellij-idea-2026-2/
- https://www.jetbrains.com/help/idea/recent-files-and-changes.html
- https://plugins.jetbrains.com/docs/intellij/tool-window.html
- https://plugins.jetbrains.com/docs/intellij/icons-style.html

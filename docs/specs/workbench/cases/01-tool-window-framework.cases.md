# Cases for 01-tool-window-framework

State notation inside `before` and `after` blocks: `anchor: [window (flags), ...]` lists visible windows of that anchor in order; `focus:` names the focused
component (`editor` or a window id); `stack:` is the side stack of an anchor, most recent first; `recent:` is the recently-used list, most recent first.
`(split)` marks the secondary half. Layout is the default Archipel profile unless `settings:` says otherwise.

### WB-TW-020 case 1: Alt+1 shows Project, second Alt+1 hides it
settings: default
language: plain
```before
left: []
bottom: []
focus: editor
```
keys: Alt+1
```after
left: [Project]
bottom: []
focus: Project
```

### WB-TW-020 case 2: second activation hides and returns focus to the editor
settings: default
language: plain
```before
left: [Project]
focus: Project
```
keys: Alt+1
```after
left: []
focus: editor
```

### WB-TW-020 case 3: activating another window leaves the first visible (different anchors)
settings: default
language: plain
```before
left: [Project]
bottom: []
focus: Project
```
keys: Alt+4
```after
left: [Project]
bottom: [Run]
focus: Run
```

### WB-TW-020 case 4: activating a visible but unfocused window focuses it, does not hide it
settings: default
language: plain
```before
left: [Project]
bottom: [Run]
focus: Run
```
keys: Alt+1
```after
left: [Project]
bottom: [Run]
focus: Project
```

### WB-TW-021 case 1: unavailable window is not activated
settings: default (no VCS root, so Commit is unavailable)
language: plain
```before
left: []
focus: editor
```
keys: Alt+0
```after
left: []
focus: editor
menu View | Tool Windows | Commit: disabled
```

### WB-TW-024 case 1: showing a second non-split window on the same anchor replaces the first
settings: default
language: plain
```before
left: [Project]
stack: left=[]
focus: Project
```
keys: Alt+0 (Commit available)
```after
left: [Commit]
stack: left=[Project]
focus: Commit
```

### WB-TW-024 case 2: a split window shares the anchor
settings: default
language: plain
```before
left: [Project]
focus: editor
```
keys: Alt+7
```after
left: [Project, Structure (split)]
focus: Structure
```

### WB-TW-025 case 1: hiding the newcomer restores the stacked window
settings: default
language: plain
```before
left: [Commit]
stack: left=[Project]
focus: Commit
```
keys: Alt+0
```after
left: [Project]
stack: left=[]
focus: editor
```

### WB-TW-025 case 2: stale stack entry is dropped
settings: default
language: plain
```before
left: [Commit]
stack: left=[Project]
event: Project is moved to the right anchor (via its stripe menu "Move to | Right"); it stays hidden
focus: Commit
```
keys: Alt+0
```after
left: []
right: []
stack: left=[]
focus: editor
```

### WB-TW-040 case 1: Shift+Escape hides the active window
settings: default
language: plain
```before
bottom: [Terminal]
focus: Terminal
```
keys: Shift+Escape
```after
bottom: []
focus: editor
```

### WB-TW-040 case 2: Shift+Escape with focus in the editor hides the last active window
settings: default
language: plain
```before
bottom: [Run]
recent: [Run]
focus: editor
```
keys: Shift+Escape
```after
bottom: []
focus: editor
```

### WB-TW-040 case 3: Shift+Escape is disabled with nothing to hide
settings: default
language: plain
```before
left: []
bottom: []
recent: []
focus: editor
```
keys: action HideActiveWindow
```after
action HideActiveWindow: disabled, no state change
```

### WB-TW-041 case 1: Escape returns focus to the editor and keeps the window open
settings: default
language: plain
```before
left: [Project]
focus: Project (tree, no speed search)
```
keys: Escape
```after
left: [Project]
focus: editor
```

### WB-TW-041 case 2: Escape closes speed search first
settings: default
language: plain
```before
left: [Project]
focus: Project (tree), speed search popup open with text "ser"
```
keys: Escape
```after
left: [Project]
focus: Project (tree), speed search popup closed
```

### WB-TW-042 case 1: Hide All and Restore
settings: default
language: plain
```before
left: [Project]
bottom: [Run]
focus: Project
```
keys: Ctrl+Shift+F12, Ctrl+Shift+F12
```after
after first press: left: [], bottom: [], focus: editor, action text "Restore Windows"
after second press: left: [Project], bottom: [Run] at their old anchors and sizes, action text "Hide All Windows" (focus after restore is unspecified)
```

### WB-TW-042 case 2: Hide All disabled with nothing visible and nothing remembered
settings: default
language: plain
```before
left: []
bottom: []
remembered layout: none
```
keys: action HideAllWindows
```after
action HideAllWindows: disabled
```

### WB-TW-044 case 1: F12 jumps back to the last tool window
settings: default
language: plain
```before
bottom: []
recent: [Run]
focus: editor
```
keys: F12
```after
bottom: [Run]
focus: Run
```

### WB-TW-050 case 1: view mode round trip keeps size
settings: default
language: plain
```before
left: [Project] weight 0.25 type docked
```
keys: action FloatMode, action DockPinnedMode
```after
after FloatMode: Project in separate dialog, type floating, left: []
after DockPinnedMode: left: [Project] weight 0.25 type docked auto-hide off
```

### WB-TW-053 case 1: Dock Unpinned hides on focus loss, Dock Pinned does not
settings: Project view mode = Dock Unpinned
language: plain
```before
left: [Project] (auto-hide)
focus: Project
```
keys: mouse click into the editor
```after
left: []
focus: editor
```

### WB-TW-070 case 1: maximize toggles and renames the action
settings: default
language: plain
```before
bottom: [Run]
focus: Run
menu text of MaximizeToolWindow: "Maximize Tool Window"
```
keys: Ctrl+Shift+QUOTE, Ctrl+Shift+QUOTE
```after
after first press: Run fills the frame center, editor area hidden, menu text "Restore Tool Window Size"
after second press: previous layout restored, menu text "Maximize Tool Window"
```

### WB-TW-071 case 1: stretch directions for a left window
settings: default
language: plain
```before
left: [Project] width 300 px
focus: Project
```
keys: Ctrl+Alt+Shift+Right, Ctrl+Alt+Shift+Left, Ctrl+Alt+Shift+Left
```after
after first: width 300 + step
after second: width 300
after third: width 300 - step
Ctrl+Alt+Shift+Up and Ctrl+Alt+Shift+Down: disabled for this window
```

### WB-TW-071 case 2: stretch directions for a bottom window
settings: default
language: plain
```before
bottom: [Run] height 250 px
focus: Run
```
keys: Ctrl+Alt+Shift+Up, Ctrl+Alt+Shift+Down
```after
after Up: height 250 + step
after Down: height 250
Ctrl+Alt+Shift+Left / Right: disabled for this window
```

### WB-TW-072 case 1: stretch actions disabled in the editor
settings: default
language: plain
```before
left: [Project]
focus: editor (regular file editor)
```
keys: action ResizeToolWindowRight
```after
action ResizeToolWindowRight: disabled and hidden, width unchanged
```

### WB-TW-074 case 1: unified size on the same anchor (New UI default)
settings: rememberSizeForEachToolWindowNewUI=false
language: plain
```before
left: [Project] width 300 px, Commit saved width 220 px (hidden)
```
keys: Alt+0
```after
left: [Commit] width 300 px
```

### WB-TW-074 case 2: per-window size remembered
settings: rememberSizeForEachToolWindowNewUI=true
language: plain
```before
left: [Project] width 300 px, Commit saved width 220 px (hidden)
```
keys: Alt+0
```after
left: [Commit] width 220 px
```

### WB-TW-008 case 1: Hide from the stripe menu removes the button, activate restores it
settings: default
language: plain
```before
left: [Bookmarks (split)], Bookmarks stripe button present
```
keys: stripe menu on Bookmarks button | Hide, then Alt+2
```after
after Hide: Bookmarks hidden, button absent
after Alt+2: Bookmarks visible and focused, button present again
```

### WB-TW-006 case 1: hidden stripes keep shortcuts working
settings: hideToolStripes=true
language: plain
```before
all stripes hidden, left: []
```
keys: Alt+1
```after
left: [Project], stripes still hidden
Notifications status widget: available
```

### WB-TW-022 case 1: focus-only activation of a visible window
settings: default
language: plain
```before
left: [Project]
focus: editor
```
keys: Alt+1
```after
left: [Project]
focus: Project
```

### WB-TW-023 case 1: show is idempotent
settings: default
language: plain
```before
bottom: [Run]
focus: editor
```
keys: API call show(Run) twice
```after
bottom: [Run] (one decorator); focus: editor (show does not move focus)
```

### WB-TW-026 case 1: floating windows do not share slots
settings: default
language: plain
```before
left: [Project]; Commit and Bookmarks are floating windows, both hidden
```
keys: show Commit (floating), show Bookmarks (floating)
```after
left: [Project]; floating: [Commit, Bookmarks] all visible at once
```

### WB-TW-027 case 1: recently used order
settings: default
language: plain
```before
recent: []
```
keys: Alt+1, Alt+4, Alt+1
```after
recent: [Project, Run]
```

### WB-TW-028 case 1: hiding a window without focus keeps focus
settings: default
language: plain
```before
bottom: [Run]
focus: editor
```
keys: stripe button menu on Run | Hide
```after
bottom: []
focus: editor
```

### WB-TW-029 case 1: focus is restored only for the active window
settings: default
language: plain
```before
project closed while: left: [Project] visible, focus Run (bottom: [Run])
```
keys: reopen the project
```after
left: [Project]; bottom: [Run]; focus: Run
```

### WB-TW-029 case 2: visible but not active
settings: default
language: plain
```before
project closed while: bottom: [Run] visible, focus editor
```
keys: reopen the project
```after
bottom: [Run]; focus: editor
```

### WB-TW-083 case 1: toggling split mode swaps slots
settings: default
language: plain
```before
left: [Project, Structure (split)]
```
keys: stripe menu on Structure | Split Mode (off)
```after
left: [Structure]   (Structure now occupies the primary slot; Project is hidden and stacked)
stack: left=[Project]
```

### WB-TW-091 case 1: restore current layout
settings: default
language: plain
```before
saved layout: left: [Project]. Current arrangement: Project moved to right, Run floating.
```
keys: Shift+F12
```after
left: [Project]; right: []; Run docked at bottom hidden as in the saved layout
```

### WB-TW-080 case 1: side-by-side option
settings: leftHorizontalSplit=true
language: plain
```before
left: [Project, Structure (split)]
```
keys: (none)
```after
Project and Structure are shown next to each other (left half / right half of the left anchor area), equal widths
```

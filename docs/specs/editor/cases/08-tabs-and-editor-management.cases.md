# Cases for 08 tabs and editor management

### ED-TABS-002 case 1: a new tab opens right of the active tab
settings: default
tabs: a.txt | [b.txt] | c.txt
keys: open d.txt
after tabs: a.txt | b.txt | [d.txt] | c.txt

### ED-TABS-002 case 2: open new tabs at the end
settings: Open new tabs at the end=on
tabs: a.txt | [b.txt] | c.txt
keys: open d.txt
after tabs: a.txt | b.txt | c.txt | [d.txt]

### ED-TABS-002 case 3: opening an open file selects its tab
settings: default
tabs: [a.txt] | b.txt | c.txt
keys: open c.txt
after tabs: a.txt | b.txt | [c.txt]

### ED-TABS-002 case 4: a new tab opens after the active tab, even if that is the first
settings: default
tabs: [a.txt] | b.txt
keys: open c.txt
after tabs: a.txt | [c.txt] | b.txt

### ED-TABS-010 case 1: closing the active tab activates the tab on the left
settings: default
tabs: a.txt | [b.txt] | c.txt
keys: Ctrl+F4
after tabs: [a.txt] | c.txt

### ED-TABS-010 case 2: activating the tab on the right
settings: When the current tab is closed, activate=The tab on the right
tabs: a.txt | [b.txt] | c.txt
keys: Ctrl+F4
after tabs: a.txt | [c.txt]

### ED-TABS-010 case 3: closing the last tab activates the one on the left
settings: When the current tab is closed, activate=The tab on the right
tabs: a.txt | b.txt | [c.txt]
keys: Ctrl+F4
after tabs: a.txt | [b.txt]

### ED-TABS-010 case 4: closing a background tab keeps the active one
settings: default
status: unverified (the keys close a tab that is not active)
tabs: a.txt | [b.txt] | c.txt
keys: close c.txt
after tabs: a.txt | [b.txt]

### ED-TABS-010 case 5: most recently opened tab
settings: When the current tab is closed, activate=Most recently opened tab
history: a.txt, c.txt, b.txt
tabs: a.txt | [b.txt] | c.txt
keys: Ctrl+F4
after tabs: a.txt | [c.txt]

### ED-TABS-010 case 6: closing the first tab
settings: default
status: unverified (the strip's default selection when the first tab closes)
tabs: [a.txt] | b.txt | c.txt
keys: Ctrl+F4
after tabs: [b.txt] | c.txt

### ED-TABS-011 case 1: Close Other Tabs keeps pinned tabs
settings: default
tabs: a.txt | [b.txt] | c.txt(p)
keys: action CloseAllEditorsButActive
after tabs: [b.txt] | c.txt(p)

### ED-TABS-012 case 1: the tab limit closes the least recently used tab
settings: Tab limit=3
history: a.txt, b.txt, c.txt
tabs: a.txt | b.txt | [c.txt]
keys: open d.txt
after tabs: b.txt | c.txt | [d.txt]

### ED-TABS-012 case 2: pinned tabs are never trimmed
settings: Tab limit=3
history: a.txt, b.txt, c.txt
tabs: a.txt(p) | b.txt | [c.txt]
keys: open d.txt
after tabs: a.txt(p) | c.txt | [d.txt]

### ED-TABS-012 case 3: the tab limit with close unchanged
settings: Tab limit=3, When tabs exceed the limit=Close unchanged
history: a.txt, b.txt, c.txt
tabs: a.txt* | b.txt | [c.txt]
keys: open d.txt
after tabs: a.txt* | c.txt | [d.txt]

### ED-TABS-005 case 1: a passive open replaces the preview tab
settings: Enable preview tab=on
tabs: a.txt | [b.txt~]
keys: open c.txt in preview
after tabs: a.txt | [c.txt~]

### ED-TABS-005 case 2: editing makes the preview tab permanent
settings: Enable preview tab=on
tabs: a.txt | [b.txt~]
keys: type "x", open c.txt in preview
after tabs: a.txt | b.txt | [c.txt~]

### ED-TABS-013 case 1: reopen a closed tab at its position
settings: default
tabs: a.txt | [b.txt] | c.txt
keys: Ctrl+F4, action ReopenClosedTab
after tabs: a.txt | [b.txt] | c.txt

### ED-TABS-013 case 2: pin state is restored too
settings: default
tabs: [b.txt(p)] | a.txt
keys: Ctrl+F4, action ReopenClosedTab
after tabs: [b.txt(p)] | a.txt

### ED-TABS-015 case 1: pinning moves the tab to the left group
settings: default
tabs: a.txt | [b.txt]
keys: action PinActiveTabToggle
after tabs: [b.txt(p)] | a.txt

### ED-TABS-015 case 2: unpinning
settings: default
tabs: [b.txt(p)] | a.txt
keys: action PinActiveTabToggle
after tabs: [b.txt] | a.txt

### ED-TABS-017 case 1: next tab
settings: default
tabs: [a.txt] | b.txt | c.txt
keys: Alt+Right
after tabs: a.txt | [b.txt] | c.txt

### ED-TABS-017 case 2: previous tab at the first position wraps
settings: default
status: unverified (wrapping default)
tabs: [a.txt] | b.txt | c.txt
keys: Alt+Left
after tabs: a.txt | b.txt | [c.txt]

### ED-TABS-020 case 1: Split Right opens the same file in a new window
settings: default
tabs: [a.txt]
keys: action SplitVertically
after tabs: [a.txt] || a.txt

### ED-NAV-002 case 1: Back after two jumps
settings: default
language: plain
```before
<caret>l0
l1
l2
l3
l4
l5
l6
l7
l8
l9
```
keys: Ctrl+G "6" Enter, Ctrl+G "10" Enter, Ctrl+Alt+Left
```after
l0
l1
l2
l3
l4
<caret>l5
l6
l7
l8
l9
```

### ED-NAV-002 case 2: Back twice reaches the start
settings: default
language: plain
```before
<caret>l0
l1
l2
l3
l4
l5
l6
l7
l8
l9
```
keys: Ctrl+G "6" Enter, Ctrl+G "10" Enter, Ctrl+Alt+Left, Ctrl+Alt+Left
```after
<caret>l0
l1
l2
l3
l4
l5
l6
l7
l8
l9
```

### ED-NAV-003 case 1: Forward after Back
settings: default
language: plain
```before
<caret>l0
l1
l2
l3
l4
l5
l6
l7
l8
l9
```
keys: Ctrl+G "6" Enter, Ctrl+G "10" Enter, Ctrl+Alt+Left, Ctrl+Alt+Right
```after
l0
l1
l2
l3
l4
l5
l6
l7
l8
<caret>l9
```

### ED-NAV-003 case 2: a hop of fewer than 4 lines replaces the previous place
settings: default
status: unverified (Go to Line is assumed to be a navigation command)
language: plain
```before
<caret>l0
l1
l2
l3
l4
l5
l6
l7
l8
l9
```
keys: Ctrl+G "3" Enter, Ctrl+G "9" Enter, Ctrl+Alt+Left
```after
l0
l1
<caret>l2
l3
l4
l5
l6
l7
l8
l9
```

### ED-NAV-005 case 1: Last Edit Location
settings: default
language: plain
```before
<caret>l0
l1
l2
l3
l4
l5
```
keys: Ctrl+G "5" Enter, type "x", Ctrl+Home, Ctrl+Shift+Backspace
```after
l0
l1
l2
l3
x<caret>l4
l5
```

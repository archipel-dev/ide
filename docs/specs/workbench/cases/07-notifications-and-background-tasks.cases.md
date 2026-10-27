# Cases for 07-notifications-and-background-tasks

Notation: `t=` is time since the event, in seconds, with the application active and the mouse away from the balloon unless stated.
`balloons:` lists visible balloons (newest first, corner stack). `timeline:` lists the Timeline tab (newest first). `badge:` is the unread count.

### WB-NOT-011 case 1: balloon expires after 10 seconds
settings: group "build" display type BALLOON, log on
language: plain
```before
no balloons; timeline empty
```
keys: post notification (group build, type information, title "Build finished", content "2 warnings")
```after
t=0:  balloons: [Build finished]
t=9:  balloons: [Build finished]
t=11: balloons: []
timeline: [Build finished]   badge: 1
```

### WB-NOT-011 case 2: hover pauses the timer
settings: group "build" display type BALLOON
language: plain
```before
balloon visible at t=0
```
keys: move the mouse over the balloon at t=5, keep it there until t=30, move away
```after
t=30: balloons: [Build finished] (still shown)
t=36: balloons: []
```

### WB-NOT-011 case 3: inactive application defers the timer
settings: group "build" display type BALLOON
language: plain
```before
application is not the foreground app when the notification is posted
```
keys: post notification; wait 60 s; activate the application
```after
at activation (t=0 of the timer): balloons: [Build finished]
10 seconds later: balloons: []
```

### WB-NOT-012 case 1: sticky balloon stays
settings: group "vcs" display type STICKY_BALLOON
language: plain
```before
no balloons
```
keys: post notification (group vcs), wait 120 s
```after
balloons: [the notification]  (still visible)
click close: balloons: []
```

### WB-NOT-013 case 1: no popup and no log
settings: group "quiet" display type NONE, log off
language: plain
```before
timeline empty
```
keys: post notification (group quiet)
```after
balloons: []; timeline: []; badge: 0
the notification is expired
```

### WB-NOT-013 case 2: no popup but logged
settings: group "quiet" display type NONE, log on
language: plain
```before
timeline empty
```
keys: post notification (group quiet, title "Quiet")
```after
balloons: []; timeline: [Quiet]; badge: 1
```

### WB-NOT-013 case 3: balloon closed by the user, group does not log
settings: group "loud" display type BALLOON, log off
language: plain
```before
balloon [Loud] visible
```
keys: click the close button
```after
balloons: []; timeline: []; the notification is expired
```

### WB-NOT-013 case 4: balloon closed by the user, group logs
settings: group "loud" display type BALLOON, log on
language: plain
```before
balloon [Loud] visible
```
keys: click the close button
```after
balloons: []; timeline: [Loud]
```

### WB-NOT-014 case 1: balloons globally off
settings: "Display balloon notifications" = off; group "build" BALLOON, log on
language: plain
```before
no balloons
```
keys: post notification (group build)
```after
balloons: []; timeline: [the notification]; badge: 1
```

### WB-NOT-002 case 1: unregistered group
settings: group "acme.unknown" not registered
language: plain
```before
Settings > Notifications table has no row "acme.unknown"
```
keys: post notification (group acme.unknown)
```after
balloons: [the notification] (type BALLOON)
Settings > Notifications table now has a row for "acme.unknown" with Popup type "Balloon"
```

### WB-NOT-020 case 1: stack limit
settings: default (2 visible), all groups BALLOON
language: plain
```before
no balloons
```
keys: post notifications N1, N2, N3, N4 quickly
```after
balloons (corner, newest first): [N4] [N3]
collapsed row: 2 more notifications
```

### WB-NOT-020 case 2: closing reveals the next
settings: as before, balloons [N4] [N3] + 2 more
language: plain
```before
stack as above
```
keys: close N4
```after
balloons: [N3] [N2]
collapsed row: 1 more notification
```

### WB-NOT-022 case 1: close all with Ctrl
settings: two balloons visible
language: plain
```before
balloons: [N2] [N1]
```
keys: Ctrl+click the close button of N2
```after
balloons: []
```

### WB-NOT-023 case 1: gear menu
settings: balloon visible for group "build"
language: plain
```before
balloon [Build finished]
```
keys: click the gear button
```after
menu entries include: Settings...
choose Settings...: Settings dialog opens on Appearance & Behavior > Notifications with the row "build" selected
```

### WB-NOT-030 case 1: timeline order and Clear all
settings: groups log on
language: plain
```before
notifications posted in order: A, B, C
Notifications tool window open, tab "Timeline"
```
keys: (none)
```after
timeline: [C] [B] [A]
```

### WB-NOT-030 case 2: Clear all
settings: as before
language: plain
```before
timeline: [C] [B] [A]
```
keys: click "Clear all"
```after
timeline: []
empty text: Suggestions, events, / and errors will appear here
```

### WB-NOT-032 case 1: unread badge
settings: stripes hidden (Notifications widget available)
language: plain
```before
badge: 0; window closed
```
keys: post two notifications, then open the Notifications window
```after
before opening: badge 2; tooltip: 2 notifications pending
after opening: badge 0; tooltip: No new notifications
```

### WB-NOT-033 case 1: action in the timeline
settings: notification with action "Retry" in the timeline
language: plain
```before
timeline: [Build failed  Retry]
```
keys: click "Retry"
```after
the Retry action runs once
```

### WB-NOT-034 case 1: suggestion suppression
settings: a suggestion notification for group "tips"
language: plain
```before
Suggestions tab: [Try feature X]
```
keys: context action "Don't Show Again For This Project"
```after
the suggestion disappears; Settings > Notifications > "Don't ask again notifications:" > "This Project" lists it; other projects still show suggestions of that group
```

### WB-NOT-041 case 1: popup type values
settings: default
language: plain
```before
Settings > Notifications table
```
keys: open the "Popup type:" combo of a normal group
```after
values: No popup, Balloon, Sticky balloon
```

### WB-NOT-040 case 1: changing a group takes effect
settings: group "build" BALLOON
language: plain
```before
Settings > Notifications: set group "build" to Sticky balloon, Apply
```
keys: post notification (group build), wait 30 s
```after
balloons: [the notification] (still visible, sticky)
```

### WB-NOT-050 case 1: determinate progress
settings: default
language: plain
```before
no background tasks
```
keys: start task "Indexing" (cancellable), set fraction 0.5 with text "src/main"
```after
status bar: "Indexing: src/main" with a half-full bar and a cancel (x) button
finish the task: the view disappears
```

### WB-NOT-051 case 1: cancel
settings: default
language: plain
```before
status bar: Indexing: src/main [bar] (x)
```
keys: click (x)
```after
status bar text: Stopping - Indexing; no cancel button
task observes cancellation at its next checkpoint and ends; the view disappears
```

### WB-NOT-051 case 2: non-cancellable task
settings: default
language: plain
```before
no background tasks
```
keys: start task "Saving" (not cancellable)
```after
status bar: Saving [bar]; no cancel button
```

### WB-NOT-052 case 1: several tasks
settings: default
language: plain
```before
no background tasks
```
keys: start tasks T1, T2, T3
```after
status bar: first task view and a link "Show all (3)"
click the link: processes popup with 3 rows; link now reads "Hide processes (3)"
click again: popup closes
```

### WB-NOT-053 case 1: pause and resume
settings: default
language: plain
```before
suspendable task "Download" running
```
keys: click pause, hover the button, click it again
```after
after pause: text Paused; tooltip Resume
after resume: work continues; tooltip Pause
```

### WB-NOT-055 case 1: modal progress and Escape
settings: default
language: plain
```before
a modal cancellable task "Searching References..." is running
```
keys: Escape
```after
dialog closes; the task is cancelled
```

### WB-NOT-056 case 1: exit confirmation with tasks
settings: default; indexing running
language: plain
```before
background task running
```
keys: File > Exit
```after
dialog "Confirm Exit": Background tasks are running. Are you sure you want to stop them and exit?
choose No: the IDE keeps running; choose Yes: tasks cancelled and the IDE exits
```

### WB-NOT-056 case 2: exit confirmation without tasks
settings: "Confirm before exiting the IDE" on; no tasks
language: plain
```before
no background tasks
```
keys: File > Exit
```after
dialog "Confirm Exit": Are you sure you want to exit?
```

### WB-NOT-057 case 1: popup does not auto-open
settings: Auto Show off
language: plain
```before
no tasks
```
keys: start tasks T1, T2
```after
processes popup closed; link "Show all (2)" visible
```

### WB-NOT-001 case 1: queued before the frame
settings: default
language: plain
```before
notification posted during startup before the frame exists
```
keys: frame appears
```after
balloon shown right after the frame
```

### WB-NOT-003 case 1: expire is idempotent
settings: default
language: plain
```before
balloon visible
```
keys: expire twice
```after
balloon gone, no error
```

### WB-NOT-010 case 1: four display types
settings: default
language: plain
```before
legacy group bound to a tool window, Settings > Notifications
```
keys: open Popup type combo
```after
No popup, Balloon, Sticky balloon, Tool window balloon
```

### WB-NOT-015 case 1: Welcome screen anchor
settings: no project open
language: plain
```before
Welcome screen
```
keys: post a project-less notification
```after
balloon in the Welcome frame
```

### WB-NOT-021 case 1: content layout
settings: default
language: plain
```before
none
```
keys: post title "Build failed", content "See log", action "Retry"
```after
rows: icon + bold "Build failed"; "See log"; link "Retry"
```

### WB-NOT-031 case 1: empty state
settings: default
language: plain
```before
no notifications
```
keys: open Notifications window
```after
Suggestions, events, / and errors will appear here
```

### WB-NOT-054 case 1: cooperative cancel
settings: default
language: plain
```before
task polling every 10 ms
```
keys: cancel
```after
task ends within 100 ms
```

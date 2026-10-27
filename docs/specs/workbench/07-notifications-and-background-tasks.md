# Workbench spec 07: notifications and background tasks

Status: specification (clean-room). ID prefix `WB-NOT`. Priorities `MUST` / `SHOULD` / `LATER` as in `docs/specs/README.md`.
Executable cases: `cases/07-notifications-and-background-tasks.cases.md`.

Scope: notification model and groups, display types and timing, balloon layout and content, the Notifications tool window (suggestions, timeline), the Notifications settings page,
the notification status widget (see also `04-status-bar.md`), and background tasks: progress shown in the status bar, the processes popup, pause and cancel, cancellation semantics, and
the exit confirmation. Dumb mode progress texts belong to `docs/specs/platform/04-dumb-mode.md`.

Sources studied: `ic: platform/platform-impl/src/com/intellij/notification/impl/NotificationsManagerImpl.java`, `NotificationsConfigurationImpl.java`, `NotificationsToolWindow.kt`, `ui/NotificationsConfigurablePanel.java`,
`ic: platform/ide-core/src/com/intellij/notification/NotificationDisplayType.java`, `ic: platform/platform-impl/src/com/intellij/ui/BalloonLayoutImpl.kt`,
`ic: platform/platform-impl/src/com/intellij/openapi/wm/impl/status/InfoAndProgressPanel.kt`, `ProcessPopup.java`, `ic: platform/platform-impl/src/com/intellij/notification/impl/widget/NotificationWidgetFactory.java`,
`ic: platform/platform-api/resources/messages/IdeBundle.properties`, `ic: platform/ide-core/resources/messages/IdeCoreBundle.properties`, `ActionsBundle.properties`, inventory `window-ui.md` (Notifications rows).

## 1. Model

A **notification** has: `group id` (string), `type` (`information`, `warning`, `error`; plus `idea` for IDE-level hints, `suggestion` flag), `title`, optional `subtitle`, `content` (a small HTML subset: text, `<b>`, `<br>`, links), a list of **actions** (links or buttons),
an optional hyperlink listener, an `important` flag (actions render as buttons), `expired` state, a timestamp, and optionally a project. Notifications are created by any component through the service; **groups** are declared with an id, a display name for the settings page,
a **default display type** and `log by default`. A notification whose group is not declared is auto-registered with the display type `BALLOON`.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-NOT-001 | MUST | A notification service accepts notifications from any thread, delivers them on the UI thread, and queues them until the frame exists (early notifications are shown right after startup, in order). When paused (`pauseNotifications`), they accumulate and show on resume in order. A notification for a project shows after that project finished opening. | A notification posted before the frame appears shows after the frame appears. Posted for a project still opening: shown only after it opened. |
| WB-NOT-002 | MUST | Groups are registered with id, name, default display type and log flag; an unregistered group id registers itself as `BALLOON`. Per-group user settings override the default (see 6). | Notify with group `acme.unknown`: shows as a balloon and the group is listed on the settings page. |
| WB-NOT-003 | MUST | `expire` removes a notification everywhere (balloon, tool window, widget count) and is idempotent; an expired notification never shows. | Call expire twice: no error; balloon closes. |
| WB-NOT-004 | MUST | Notification types map to severity icons and colors: information, warning, error; errors may use the error accent in the timeline. | The three types render distinct icons. |

## 2. Display types and timing

| Display type (settings label) | Behavior |
|---|---|
| `NONE` ("No popup") | no balloon; only the log (timeline) if the group logs |
| `BALLOON` ("Balloon") | a balloon in the frame; **auto-closes after 10 seconds** (a *smart fade-out* timer that starts when the application is active and pauses while the mouse is over the balloon) |
| `STICKY_BALLOON` ("Sticky balloon") | a balloon that stays until the user closes it; fade-out after **300 000 ms** (5 minutes) |
| `TOOL_WINDOW` ("Tool window balloon") | legacy: a message in the tool window named by the notification (or its group's tool window); **falls back to BALLOON** when no tool window can show it |

Decision procedure (in order): (1) group not registered: register as BALLOON. (2) if the group does **not** log, and (display type `NONE`, or the notification suppresses popups, or balloons are disabled globally) the notification expires at once, without a trace.
(3) if balloons are enabled globally and a project/frame exists, the display type decides: `NONE` shows nothing; a balloon type shows the balloon; `TOOL_WINDOW` as above. (4) A notification of a logging group stays in the **timeline** of the Notifications tool window after its balloon is gone, until cleared.
(5) For a balloon in a project window, when the balloon is **closed by the user or expires** and the group does not log, or the type is sticky, the notification is expired (removed). With logging on, a non-sticky balloon's notification remains in the timeline.
When the frame is inactive the timer starts at the next activation.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-NOT-010 | MUST | Display types and their texts are exactly the four above. | The settings combo for a group lists: No popup, Balloon, Sticky balloon, Tool window balloon. |
| WB-NOT-011 | MUST | A `BALLOON` closes automatically after 10 s of application-active, no-hover time; hovering pauses the timer; deactivating the app pauses it. | Show a balloon, no interaction: it disappears at ~10 s. Hover at 5 s for 20 s: still shown; move away: closes ~5 s later. |
| WB-NOT-012 | MUST | A `STICKY_BALLOON` stays until closed (the 5 minute fade-out is a safety limit). | After 60 s it is still shown. |
| WB-NOT-013 | MUST | The decision procedure above, including silent expiry of non-logging, popup-less notifications. | Group with type NONE and log off: no balloon, no timeline entry. With log on: no balloon, entry in the timeline. |
| WB-NOT-014 | MUST | "Display balloon notifications" (global, default on): when off no balloons are shown at all; logging groups still collect in the timeline. | Off: a notification produces no balloon but appears in the timeline (if it logs). |
| WB-NOT-015 | MUST | The balloon is anchored to the frame the notification belongs to: the project's frame, otherwise the active frame, otherwise the Welcome screen. | A project-less notification shows on the Welcome screen when no project is open. |

## 3. Balloon layout and content

Balloons stack in a corner of the frame (default: **bottom right**; the user can choose Top Right, Bottom Right, Top Left, Bottom Left; "default" means bottom right). At most **2** balloons are visible at once (`ide.notification.visible.count` = 2); more are collapsed into a
summary balloon "{n} more {notifications|suggestions}" (kinds: "notifications" for timeline items, "suggestions" for suggestion items); new notifications push older ones up.

Content: icon (type), **title** (bold), optional **subtitle**, **content** (wraps; long content is truncated with an expand control), then the **actions** as blue links (or as buttons if `important`), with a **"More" dropdown** when there are too many to fit; a **close** button ("x", tooltip "Close. {key}click to close all notifications",
Ctrl+click or Cmd+click closes all) and a **gear** (tooltip "Turn off or change behavior") that opens a menu with **"Settings..."** (opens the Notifications page filtered to the group) and **"Don't show again"** for notifications that support it (effect: open question 2).
Pressing the action runs it and, unless the action keeps the notification, expires the notification.

Notification actions in the Notifications group: `CloseAllNotifications` ("Close All"), `CloseFirstNotification` ("Close First"), `ClearAllNotifications` ("Clear All Notifications"), `MarkAllNotificationsAsReadAction` ("Mark All Notifications as Read", description "Mark all unread notifications as read"), `MarkNotificationsAsRead`.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-NOT-020 | MUST | Balloon corner follows the setting; stacking order puts the newest at the corner; at most 2 are visible, the rest collapse into "{n} more notifications" (or "suggestions"); closing one reveals the next. | Post 4 notifications: two balloons plus the collapsed row "2 more notifications". |
| WB-NOT-021 | MUST | Balloon content: icon, bold title, optional subtitle, content, actions as links; hyperlink clicks in content invoke the listener. | A notification with title "Build failed", content "See log" and one action "Retry": three text rows and a link "Retry". |
| WB-NOT-022 | MUST | The close button expires the balloon (and, when the group logs, keeps the timeline entry); Ctrl/Cmd+click closes all visible balloons. | Click x: balloon gone; entry remains in the timeline for a logging group. |
| WB-NOT-023 | MUST | The gear menu offers "Settings..." (and "Don't show again" where applicable). "Settings..." opens Settings > Appearance & Behavior > Notifications focused on the notification's group. | Click gear > Settings...: dialog on Notifications page, the group row selected. |
| WB-NOT-024 | SHOULD | `Close All` (menu entry in the notification balloon's menu) closes every visible balloon; `Close First` closes the first. | Two balloons, Close First: one remains. |
| WB-NOT-025 | SHOULD | Balloons never steal keyboard focus; Escape does not close them; they are readable by screen readers when "Announce notifications with screen reader" is on. | Typing in the editor continues uninterrupted while a balloon appears. |

## 4. Notifications tool window

Id `Notifications`, title "Notifications", anchor `right`, split (secondary), no default shortcut (see `01-tool-window-framework.md`). Two tabs:

* **Suggestions** (suggestion-type notifications: tips, plugin recommendations, announcements; they stay until acted on or dismissed),
* **Timeline** (every logged notification, newest first, grouped by day; entries show icon, title, content, actions and the time).

Toolbar: **Clear all** (`notifications.toolwindow.timeline.clear.all`, text "Clear all", clears the Timeline), **Mark All Notifications as Read**. Entry context actions for suggestions: **Remind me tomorrow**, **Don't Show Again For This Project**, **Don't Show Again**.
Empty state: two centered lines "Suggestions, events," and "and errors will appear here". An **unread badge** appears on the tool window icon (and on the Notifications status widget) while unread entries exist; opening the window marks them as read.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-NOT-030 | MUST | The window exists with tabs "Suggestions" and "Timeline"; the Timeline lists logged notifications newest first; Clear all empties it. | Three logged notifications: three rows, newest on top; Clear all: empty state. |
| WB-NOT-031 | MUST | Empty state text: "Suggestions, events," / "and errors will appear here". | New profile: the text shows. |
| WB-NOT-032 | MUST | Unread count: a new logged notification increments it; opening the window (or Mark All as Read) sets it to 0; the tool window icon and the status widget show the count as a badge. | Notify twice: badge 2; open window: badge gone. |
| WB-NOT-033 | MUST | A notification action in the window runs the action; expiring a notification removes its row; clicking a hyperlink runs its listener. | Click "Retry" in the timeline: action runs. |
| WB-NOT-034 | SHOULD | Suggestions: "Remind me tomorrow" hides the suggestion until the next day; "Don't Show Again" stores a per-group suppression; "Don't Show Again For This Project" stores a per-project one (listed in Settings under "Don't ask again notifications:" > "This Project"). | Choose "Don't Show Again" on a suggestion: group never shows suggestions again. |
| WB-NOT-035 | MUST | Entries are kept per project; project-less notifications appear in every project window's timeline until cleared. | n/a |
| WB-NOT-036 | LATER | Date grouping headers and entry collapsing for long content. | n/a |

## 5. Status widget

The Notifications status widget (see `04-status-bar.md` WB-SB-065) is only available when tool window bars are hidden or presentation mode is on. Tooltip `{n} notification(s) pending` / `No new notifications`; click opens the window (ids per WB-SB).

## 6. Settings page: Notifications

Settings id `reference.settings.ide.settings.notifications`, "Notifications", application scope, under Appearance & Behavior. Contents:

| Control | Text | Default | Meaning |
|---|---|---|---|
| checkbox | **Display balloon notifications** | on | global balloon switch (WB-NOT-014) |
| checkbox | **Enable system notifications** | on where supported | also post OS notifications |
| combo | **Notification location:** | `default` (Top Right / Bottom Right / Top Left / Bottom Left; default = Bottom Right) | balloon corner |
| table | columns **Group**, **Popup type:**, **Log**, **Read aloud**, (**Show in tool window**, **Play sound** for legacy) | per group: the group's declared display type, Log on | per-group overrides |
| empty text | "No notifications configured" | | when no group exists |
| list | **Don't ask again notifications:** with a sub-list "This Project" | empty | suppressions made from balloons |
| checkbox | **Announce notifications with screen reader**; combo **Announcement mode:** (values "With medium priority", "With high priority", "After the current output finishes", "Interrupt the current output"; comment "Notifications will interrupt previous output if it has lower priority") | off | accessibility |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-NOT-040 | MUST | The page has the controls above; the group table lists every registered group (sorted by group name) with its effective type; edits apply on OK/Apply to the next notification. | Set group "X" to "Sticky balloon", Apply: the next X notification is sticky. |
| WB-NOT-041 | MUST | The popup-type combo offers exactly "No popup", "Balloon", "Sticky balloon" (and "Tool window balloon" only for groups that bind a tool window). | A normal group offers three values. |
| WB-NOT-042 | MUST | The page is searchable by group names (search option group "{0} notifications" style). | Search "Build": the page appears with the build group row highlighted. |
| WB-NOT-043 | SHOULD | "Log" off makes notifications of that group non-persistent (WB-NOT-013). | n/a |

## 7. Background tasks and progress

A **background task** is a cancellable (or not) unit of work with a **progress indicator**: `title`, `text` (current detail), `fraction` in `[0, 1]` or **indeterminate**, `cancellable`, optional **suspender** (pausable). A task is either **modal** (a progress dialog blocks the window with the title, text, bar and "Cancel"; optionally a "Background" button that turns it into a background task) or **background** (non-blocking, shown in the status bar).

Status bar presentation (left, after the status text): for the **first** running background task, a one-line view: a short title text (`title` or `title: text` when text is set), a small progress bar (about 104 px wide when inline), and on its right a **pause** button (only if suspendable; tooltip "Pause", after pausing "Resume", text "Paused"), and a **cancel** (x) button (only when cancellable and not already stopping).
After Cancel is clicked the text becomes **"Stopping - {title}"** until the task finishes. When more than one task runs, a link **"Show all ({n})"** replaces the secondary views and opens the **processes popup**; the popup lists every task with its own bar, text, pause and cancel; its toggle reads **"Hide processes ({n})"** while open.
Actions: `ShowProcessWindow` (menu text "Show", full text "Show Processes", description "Show/Hide the processes window"), `AutoShowProcessWindow` ("Auto Show", "Show the processes window on starting of any progress"; off by default). Finished tasks disappear after a short delay (progress removals are merged in 50 ms batches).

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-NOT-050 | MUST | Background tasks appear in the status bar progress area with title/text and bar; a determinate bar shows `fraction`, an indeterminate bar animates; the view disappears when the task finishes. | Start a task with fraction 0.5: bar half full; finish: view gone within 100 ms. |
| WB-NOT-051 | MUST | A cancellable task shows a cancel (x) button; clicking it requests cancellation (the work observes it at its next checkpoint), the text becomes `Stopping - {title}` and the button disappears; a non-cancellable task has no cancel button. | Click x on "Indexing": text "Stopping - Indexing", button gone. |
| WB-NOT-052 | MUST | With two or more tasks the status bar shows one task plus the link "Show all ({n})"; clicking opens the processes popup listing all of them; the link then reads "Hide processes ({n})". | Three tasks: link "Show all (3)"; click: popup with three rows; link "Hide processes (3)". |
| WB-NOT-053 | SHOULD | Pause/Resume for suspendable tasks: pause sets text "Paused" and halts the work; resume continues; the button tooltip flips between "Pause" and "Resume". | Pause a suspendable task: tooltip "Resume", text "Paused". |
| WB-NOT-054 | MUST | **Cancellation is cooperative:** cancel marks the indicator canceled; all long-running work polls; no work outlives cancellation by more than one checkpoint (`docs/specs/platform/06-concurrency-and-cancellation.md`). | A task polling every 10 ms stops within 100 ms of cancel. |
| WB-NOT-055 | MUST | A modal progress dialog blocks input to the owner window, shows the title in the dialog title and the text/bar, has a **Cancel** button (when cancellable) and Escape cancels it (when cancellable). | Start a modal cancellable task: dialog with Cancel; Escape: cancelled. |
| WB-NOT-056 | MUST | **Exit with tasks:** quitting while background tasks run shows "Confirm Exit": "Background tasks are running. Are you sure you want to stop them and exit?" (with no tasks and "Confirm before exiting the IDE" on: "Are you sure you want to exit?"). Yes cancels the tasks and exits; No stays. | Indexing running, File > Exit: dialog above. |
| WB-NOT-057 | SHOULD | The processes popup does not auto-open by default; with "Auto Show" on it opens when more than one task is running. | Default: starting 3 tasks does not open the popup. |
| WB-NOT-058 | SHOULD | The progress area is accessible: bar and text readable, buttons focusable. | n/a |
| WB-NOT-059 | MUST | Task completion that must tell the user something does so through a notification (balloon), never only through the transient progress view. | A failed import posts an error notification. |

## 8. Inventory coverage (`window-ui.md`, notifications rows)

Tool window `Notifications` (right, secondary), actions `CloseAllNotifications`, `CloseFirstNotification`, `MarkNotificationsAsRead`, `BackgroundTasks` actions `ShowProcessWindow`, `AutoShowProcessWindow` are covered here. The status widgets `Notifications` and `FatalError` are in `04-status-bar.md`.
`ClearAllNotifications` is in the Notifications action group (covered). Run-console "Process 'x' Is Running" dialogs (Terminate / Disconnect, "Do you want to terminate the process 'x'?") belong to the run area; the System Settings option for them is in `05-settings-dialog-and-keymap.md`.

## 9. Open questions

1. Whether the smart fade-out timer restarts after hover or continues with the remaining time (assumed: continues).
2. What "Don't show again" does exactly per notification kind (group popup type to "No popup" vs a notification-specific suppression); needs a reference run.
3. Entry grouping and sort within Suggestions; exact day-header texts in the Timeline.
4. The default value and presence of "Enable system notifications" on each OS.

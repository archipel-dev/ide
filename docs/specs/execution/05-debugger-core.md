# 05. Debugger core: sessions, breakpoints, stepping, frames, variables, watches, evaluation

Status: specification. Requirement prefix `EXE-DBG`. Cases: `cases/05-debugger-core.cases.md`.
The generic part (`xdebugger`) is language independent; the Java part (JDWP) is described as
observable behavior. Archipel implements the Java debugger over JDWP (`EXE-DBG-110..`) and exposes
the generic contract so other debug backends can plug in later.

ic: `platform/xdebugger-api/src/com/intellij/xdebugger/**`,
`platform/xdebugger-impl/src/com/intellij/xdebugger/impl/**`,
`platform/xdebugger-impl/ui/resources/intellij.platform.debugger.impl.ui.actions.xml`,
`platform/xdebugger-api/resources/messages/XDebuggerBundle.properties`,
`platform/xdebugger-impl/resources/messages/XDebuggerImplBundle.properties`,
`platform/xdebugger-impl/ui/resources/messages/XDebuggerUiBundle.properties`,
`java/debugger/**` (`DebuggerSettings.java`, `ViewsGeneralSettings.java`, `ClassRenderer.java`, `ArrayRenderer.java`),
`java/debugger/openapi/resources/messages/JavaDebuggerBundle.properties`,
`platform/platform-resources/src/keymaps/$default.xml`,
`platform/platform-resources-en/src/messages/ActionsBundle.properties`.

## 1. Sessions and the Debug tool window

A **debug session** is created by starting a configuration with the Debug executor. The session
has a state: *starting* (connecting), *running*, *paused* (suspended at a position with a stack),
*stepping*, *stopped*. Only one *selected* thread and *selected frame* exist per session.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-DBG-001 | MUST | Debug tool window (id `Debug`, `Alt+5`), tab per session with the tab title the configuration name; content tabs `Debugger` (title) and `Console`. The `Debugger` tab holds the panes `Frames`, `Threads` (combined by default: the `Threads & Frames` layout `Combined`; other layouts `Threads`, `Side-by-Side`, `Show Frames Only`), `Variables` and `Watches`. The toolbars are: top (section 8 `XDebugger.ToolWindow.TopToolbar3`), left (`Resume`, `Pause`, `Stop`, `View Breakpoints`, `Mute Breakpoints`). | Debugging `Main` shows tab `Main` with `Debugger` and `Console`. |
| EXE-DBG-002 | MUST | Empty state text of the Debug tool window: `To debug your code, do one of the following:` then `— Click the Run icon in the editor gutter` and `— Select "Debug..." in the editor context menu`, link `Debugging code`. | Opening the Debug window without a session shows this text. |
| EXE-DBG-003 | MUST | Session status line texts (status bar / pane): `Connected` / `Connecting` / `Disconnected` for connections; for Java stop reasons see section 10. When the session pauses the window is shown unless the setting `Show debug window on breakpoint` (default on) is off; the IDE window is brought to front unless `Focus application on breakpoint`... see section 7. | Hit breakpoint: Debug window and editor at the line. |
| EXE-DBG-004 | MUST | On pause, the editor opens the source file at the execution line, the line gets the execution highlight (blue line background with the execution arrow), the caret moves there, and the view scrolls so the line is visible (centered only when `Scroll execution point to center`, default off). If the source is not found a decompiled/stub view or the `Sources not found` state is shown (see `docs/specs/java`). `Show Execution Point` (`Alt+F10`) returns to it. | After a step the editor shows the new line. |
| EXE-DBG-005 | MUST | Stopping (`Stop`, `Ctrl+F2`) ends the session: the debuggee is terminated (or detached for attach mode, with `Disconnected from the target VM, address: '...', transport: '...'` printed), the tab remains with the console, execution highlights disappear. Setting `Hide debug window on process termination` (default off) hides it. Breakpoints muted during the session are unmuted when `Unmute Breakpoints on Session Finish` holds (default off). | Stop hides the execution line. |
| EXE-DBG-006 | MUST | `Rerun` (`Ctrl+F5`) in the Debug window restarts the configuration in debug mode (icon `Rerun in Debug Mode` in dashboards). | Rerun launches a new session. |

## 2. Breakpoints: types and properties

A breakpoint belongs to a **type**; the common properties:

| Property | UI label | Default |
|---|---|---|
| enabled | `Enabled` (dialog mnemonic `d`) | on |
| suspend policy | `Suspend:` `All` / `Thread`; `Suspend execution` check (off means policy none) | `All` |
| condition | `Condition:` (expression editor, language of the file) | none |
| log message | `"Breakpoint hit" message` (text printed to the console) | off |
| log stack | `Stack trace` | off |
| log expression | `Evaluate and log:` | none |
| remove once hit | `Remove once hit` | off (on for *temporary* breakpoints) |
| hit count (pass count) | `Hit count:` mode `Equal to`, `Greater than or equal to`, `Is a multiple of` | off, count 0 |
| dependency (master) | `Disable until hitting the following breakpoint:` (`<None>`) with `After hit:` `Disable again` / `Leave enabled` | none, `Leave enabled` |
| filters (Java) | `Class filters:`, `Instance filters:`, `Caller filters:` (and `Catch class filters:` for exceptions) | none |
| name/description | `Breakpoint name`, `Edit Description` | none |
| group | `Move to Group`, `Create New...`, `<No Group>` | none |

Suspend policies: `Suspend: all` (all threads stop), `Suspend: thread` (only the hitting thread),
`Suspend: none` (never suspends, only logs). The default for new line, method, field and exception
breakpoints is *all threads*; the user may change the default per type with `Make Default`
(`Make De&fault`) in the dialog.

Java breakpoint types: **line** (also *lambda* and *line and lambda* variants when the line
contains lambdas), **method** (`Method entry` / `Method exit`, emulated flag), **field watchpoint**
(`Field access`, `Field modification`), **exception** (`Caught exception`, `Uncaught exception`,
class and catch-class filters), **class prepare** and others are LATER.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-DBG-010 | MUST | Line breakpoints: toggled by `Toggle Line Breakpoint` (`Ctrl+F8`) at the caret line, by click on the gutter, and by the hover `+` ("Add Breakpoint" tooltip). A line can have one line breakpoint plus lambda variants. Allowed only on lines with executable code (otherwise the breakpoint is *invalid*: icon with a cross; hover message `Breakpoint does not belong to any class` or the source-mismatch messages `Source file changed or found source file does not correspond to the breakpoint's file`, `The class {0}, found in the project classpath, has no associated source files`). | Case EXE-DBG-010 case 1..3. |
| EXE-DBG-011 | MUST | Breakpoint position follows edits: inserting lines above moves the breakpoint with its line; deleting the line removes it; the position is stored as file URL and line (zero-based), persisted in the project workspace (`.idea/workspace.xml`, component `XDebuggerManager`, one `line-breakpoint` element with `url`, `line`, `option` children), restored on project open. Breakpoints are saved immediately on change. | Insert a line above: breakpoint stays on the same statement. |
| EXE-DBG-012 | MUST | Gutter rendering: enabled = red filled circle; disabled = grey circle with red border; with condition = circle with a question mark; with log expression/message only (suspend none) = small dot; temporary = circle with a clock/arrow (`Toggle Temporary Line Breakpoint`, `Ctrl+Shift+Alt+F8`); dependent = circle with a lock; invalid = crossed circle; verified/unverified states for remote backends. The line also gets a faint red background. Setting `Breakpoints Over Line Numbers` draws them over the line number column. | Conditional breakpoint shows the question circle. |
| EXE-DBG-013 | MUST | Gutter tooltip (hover) lists, one per line when present: `Suspend: all|thread|none`, `Condition: <expr>`, `Log message: yes`, `Log stack: yes`, `Log expression: <expr>`, `Thread dump: once per session`, `Depends on: <breakpoint>`, and at the end `Right-click to edit`. Several breakpoints on one position: `Multiple breakpoints`. | Case EXE-DBG-013 case 1. |
| EXE-DBG-014 | MUST | Right click on a breakpoint opens the quick properties popup (`Edit Breakpoint`, `Ctrl+Shift+F8` with the caret on the line) with: `Enabled`, `Suspend`, `Condition`, `More` link to the dialog (`xbreakpoints.popup.more.label` `More`). Applying changes with `Done`. | Case EXE-DBG-014 case 1. |
| EXE-DBG-015 | MUST | Condition: evaluated in the frame of the hit each time; truthy (boolean true) suspends; false continues; a non-boolean value gives the error `'boolean' value expected in condition`; an evaluation error suspends and shows the dialog `Breakpoint Condition Error` with the message. | Case EXE-DBG-015 case 1..3. |
| EXE-DBG-016 | MUST | Log message / log expression / log stack: when the breakpoint is hit (condition true) the text `Breakpoint reached at <Class>.<method>(<File>:<line>)` (message) is printed to the console, the expression result printed on its own line, the stack trace after; evaluation errors show `Breakpoint Action Error`. With suspend policy none nothing stops. | Case EXE-DBG-016 case 1. |
| EXE-DBG-017 | MUST | Hit count: modes `Equal to N` (suspends only at the N-th hit), `Greater than or equal to N` (from the N-th hit on), `Is a multiple of N` (every N-th hit); the count is evaluated after the condition. Counting restarts with each session. | Mode `Is a multiple of 3`: stops at hits 3, 6, 9. |
| EXE-DBG-018 | MUST | `Remove once hit`: the breakpoint is deleted after its first suspend. Temporary line breakpoints are the same flag on creation. | After the hit the gutter icon is gone. |
| EXE-DBG-019 | SHOULD | Dependency: a dependent breakpoint is inactive (lock icon) until the master breakpoint is hit; after it has been hit once it is active. `After hit: Disable again` makes it inactive again once it has been hit; `Leave enabled` keeps it active. | Case EXE-DBG-019 case 1. |
| EXE-DBG-020 | MUST | Method breakpoints (`Toggle Method Breakpoint`, on a method declaration line): `Method entry`, `Method exit` (both default on), `Emulated` flag; suspend texts `Method 'X' entered at File(Class:line)` and `Method breakpoint reached. Method 'X' is about to exit at ...`. Field watchpoints (`Toggle Field Watchpoint`, on a field): `Field access` and `Field modification` (default modification on); texts `{Class@id}.field will be accessed at ...`, `... will be modified at ... Current value = 'a'. New value = 'b'`; static variants omit the instance. Exception breakpoints: `Any exception` plus user classes; `Caught exception`/`Uncaught exception` both on by default for new ones; `Notifications`; class filters include/exclude patterns. | Case EXE-DBG-020 case 1..3. |
| EXE-DBG-021 | MUST | Muting: `Mute Breakpoints` (toggle, left/top toolbar; text `Mute Breakpoints`) makes the session ignore all breakpoints (gutter icons grey) without changing them. State is per session; reset at session end only with `Unmute Breakpoints on Session Finish`. | Muted run does not stop at breakpoints. |
| EXE-DBG-022 | MUST | Removing: clicking an enabled breakpoint icon removes it; a breakpoint with condition or log expression asks for confirmation only when `Confirm removal of conditional or logging breakpoints` (default off) with title `Confirm Removal` and the text `Breakpoint contains` + `condition:` / `log expression:`. Dragging an icon out of the gutter removes it (setting `Remove breakpoint:` `Click with left mouse button` / `Drag to the editor or click with middle mouse button`). `Restore Breakpoint` (Run | Debugging Actions | Toggle Breakpoint) restores the last deleted. | Case EXE-DBG-022 case 1. |
| EXE-DBG-023 | MUST | `Toggle Breakpoint Enabled` enables or disables the breakpoint at the caret line; gutter popup items: `Remove other breakpoints`, `Disable other breakpoints`; `Remove All Breakpoints` and `Remove All Breakpoints In The Current File` are available as actions. Disabled breakpoints never suspend, log or count. | Disable then run: no stop. |
| EXE-DBG-024 | MUST | Gutter popup (right click on the gutter) offers the Debug sub-menu: `Toggle Line Breakpoint`, `Toggle Method Breakpoint`, field watchpoint on fields, `Toggle Temporary Line Breakpoint`. The "Add" hover menu offers `Add Breakpoint`, `Add Conditional Breakpoint...`, `Add Logging Breakpoint...`. | Hover on a free gutter line shows the faint `+`. |

### 2.1 Breakpoints dialog (`Run | View Breakpoints...`, `Ctrl+Shift+F8`)

Title `Breakpoints`. Left tree (accessible name `Breakpoints`) grouped by *Type* (`Java Line Breakpoints`,
`Java Method Breakpoints`, `Java Field Watchpoints`, `Java Exception Breakpoints`, ...), optionally by
file (`Group by file`) and user groups (`Show user groups`); each row is the checkbox (enabled) plus
the icon and `<file>:<line>` (`xbreakpoint.default.display.text` `{file}:{line}`, with column `{file}:{line}:{col}`) or the name.
Right: properties panel for the selected breakpoint with the controls of the table (label `Select a breakpoint`
when none is selected). Toolbar: add (`+` menu with types), remove, `Group by`, `Move to Group`
(`Create New...`, `New group name`), `Set as Default`/`Unset as Default`, `Edit Description`, `Done`.
The tree tab list also contains `Java Exception Breakpoints` entries with `Any exception` always present.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-DBG-030 | MUST | The dialog behaves as above; changes apply immediately to the live breakpoints (no Apply button) and persist; `Esc`/`Done` closes it. Selecting a row and `F4`/double-click navigates to the source. `Delete` removes the selected breakpoints. | Case EXE-DBG-030 case 1. |
| EXE-DBG-031 | MUST | Exception breakpoint `Any exception` is a default breakpoint: it cannot be deleted, only enabled/disabled and configured (default disabled). Adding an exception breakpoint asks for the class (`Add Exception Breakpoint`, chooser with qualified name). | `Any exception` always listed. |
| EXE-DBG-032 | SHOULD | `Add Field Watchpoint` asks `Fully qualified name of a class:` and `Field name:`; `Add Wildcard Method Breakpoint` asks `Class pattern:` and `Method name:`. | Not applicable to the first release. |

## 3. Execution control and stepping

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-DBG-040 | MUST | `Resume Program` (`F9`) resumes all threads (or only the current one with the setting `Resume only current thread`, default off) and runs until the next stop. Text `Resume Pro_gram`, description `Resume program execution`. `Pause Program` (no shortcut, text `Pause Program`) suspends the running program; after Pause expression evaluation may fail with `Couldn't evaluate expression after 'Pause Program'. Consider suspending a program with a breakpoint.` | Resume at a breakpoint runs to the next one. |
| EXE-DBG-041 | MUST | `Step Over` (`F8`): execute the current line and stop at the next line of the same method that executes; calls on the line run without stopping (breakpoints inside still stop, status text `Stepping over`); at the end of a method it stops in the caller after the call returns (the caller line is shown again). Description `Step to the next line in this file`. | Case EXE-DBG-041 case 1..3. |
| EXE-DBG-042 | MUST | `Step Into` (`F7`): if the line calls a method that has line information and is not excluded by stepping filters, stop at its first line; otherwise behave as Step Over. With `Always do smart step into` (default on) and several calls on the line, a `Method to Step Into` popup is shown (see smart step into). Status text `Stepping into`. | Case EXE-DBG-042 case 1..2. |
| EXE-DBG-043 | MUST | `Smart Step Into` (`Shift+F7`): a popup titled `Method to Step Into` lists the calls on the current line in execution order (including lambdas and constructors); text `Choose a method to step into with arrow keys.` + newline + `To exit the 'smart step into' mode, press Esc.` is shown as a hint; `Enter` steps into the selected; `Esc` cancels; the selected call is highlighted in the editor (`Smart step into target`/`selection` colors). Calls already skipped by filters are listed too. When only one candidate exists the step proceeds directly. | Case EXE-DBG-043 case 1. |
| EXE-DBG-044 | MUST | `Step Out` (`Shift+F8`): run until the current method returns and stop in the caller (at the call line after the return). Status `Stepping out`. At the top frame of the thread, resumes. | Case EXE-DBG-044 case 1. |
| EXE-DBG-045 | MUST | `Run to Cursor` (`Alt+F9`): runs until the caret line in the current editor is reached (any thread; if another thread reaches it the status is `Suspended in another thread`); other breakpoints along the way still stop. Status `Run to cursor` (`Run to cursor in {0}`). The same via click on the line number gesture when `Click line number to perform run to cursor` (default on). `Force Run to Cursor` (`Ctrl+Alt+F9`) additionally ignores all breakpoints on the way. | Case EXE-DBG-045 case 1..2. |
| EXE-DBG-046 | MUST | `Force Step Over` (`Alt+Shift+F8`): step over with breakpoints disabled for the step (description `Step over, disable all breakpoints before stepping.`); `Force Step Into` (`Alt+Shift+F7`): step into ignoring stepping filters for libraries, constructors, synthetic, simple getters, class loaders (description `Step into, ignore stepping filters for libraries, constructors, etc.`). | Case EXE-DBG-046 case 1. |
| EXE-DBG-047 | MUST | **Stepping filters** (Settings, Build, Execution, Deployment | Debugger | Stepping): `Do not step into the classes` list with checkboxes; default entries (all enabled): `com.sun.*`, `java.*`, `javax.*`, `org.omg.*`, `sun.*`, `jdk.internal.*`, `junit.*`, `org.junit.*`, `com.intellij.rt.*`, `com.yourkit.runtime.*`, `com.springsource.loaded.*`, `org.springsource.loaded.*`, `javassist.*`, `org.apache.webbeans.*`, `com.ibm.ws.*`, `org.mockito.*`, `com.jetbrains.internal.IoOverNio*`, `com.azul.*`. Options (defaults): `Skip synthetic methods` on, `Skip constructors` off, `Skip class loaders` on, `Skip simple getters` off, `Always do smart step into` on, `Hide stack frames using stepping filters` on. A step into a filtered method continues stepping until non-filtered code (or the end of the method). | Case EXE-DBG-047 case 1. |
| EXE-DBG-048 | MUST | `Show Execution Point` (`Alt+F10`) moves the editor caret to the current execution line of the selected frame and brings the Debug window's frame selection to the top frame when needed. | After scrolling away, Alt+F10 returns. |
| EXE-DBG-049 | SHOULD | `Reset Frame` (`Drop Frame`; text `Reset Frame`; description `Move the execution point back to the method call dropping current method frames from the stack`; help text `Go back to the previous frame. This will reset local variables, but the global state will not be changed.`): pops the selected frame (and those above) so execution returns to the call site; unavailable for the bottom/native frame and for frames whose caller is not available; side effects of already executed code remain; `finally` blocks follow the setting `Evaluate finally blocks on pop frame and early return` (`Always`/`Never`/`Ask`, default `Ask`). | Case EXE-DBG-049 case 1. |
| EXE-DBG-050 | SHOULD | `Force Return` (`Force Return`; description `Force a method to return before it reaches a return statement`) asks for the return value expression (void methods none); `Throw Exception` (`Throw an exception`) asks for the exception expression. Both are Java-only. | After Force Return the caller continues with the value. |
| EXE-DBG-051 | SHOULD | `Step Out of Code Block`, `Step Over Instruction`, `Trace Calls` are LATER (listed in the inventory). | Not applicable. |

## 4. Threads and frames

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-DBG-060 | MUST | The `Frames` pane shows the stack of the selected thread, top frame first, each row `method(), Class` (italic/grey for library or filtered frames) with `File.java:line` (`Show source file name`, `Show class name`, `Show package name`, `Show line number`; `Show method arguments types` off). Selecting a frame moves the editor to its line (non-top frames highlighted with a lighter `Not top frame` background) and updates Variables and Watches. A frame loading shows `Loading...`; an invalid one `<invalid frame>`; with no stack `Frames are not available`. | Case EXE-DBG-060 case 1. |
| EXE-DBG-061 | MUST | Library frames: hidden by default (`Show/Hide Library Frames`; tooltips `Hide Frames from Libraries` / `Show All Frames`); consecutive hidden frames collapse into one grey row `N hidden frames` (`1 hidden frame`) which expands on click. Frames matching stepping filters are hidden when `Hide stack frames using stepping filters` is on. | 5 frames from `java.*` collapse into `5 hidden frames`. |
| EXE-DBG-062 | MUST | The thread selector lists threads (name, state `RUNNING`, `SLEEPING`, `WAIT`, `MONITOR`, `ZOMBIE`, `NOT_STARTED`, `UNKNOWN`, with the suspend state); the thread that stopped is selected; `Move current thread to the top`, `Show thread groups` (default off; settings). Selecting a thread shows its stack; for a running thread `Frames not available for unsuspended thread`. | Case EXE-DBG-062 case 1. |
| EXE-DBG-063 | SHOULD | Thread actions: `Suspend Thread`, `Resume Thread`, `Suspend Other Threads`, `Resume All Threads`, `Freeze`/`Interrupt`/`Resume` in the Threads view; `Export Threads...`; `Copy Stack`; `Get Thread Dump` (`Dump Threads`). | Copy Stack puts the textual stack on the clipboard. |
| EXE-DBG-064 | MUST | Multiple breakpoint hits: when another thread also hits a breakpoint while stopped the message `Another thread also reached a breakpoint` (or `{0} other threads also reached breakpoints`) is shown; `Switch to the Next Stopped Breakpoint` switches context; a breakpoint hit in another thread while stepping is skipped with `Skipped breakpoint at {0} because of stepping in another thread`. | Two threads hit; message shown. |

## 5. Variables, watches, inline values

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-DBG-070 | MUST | The `Variables` pane shows, for the selected frame: `this` (instance methods), parameters and local variables visible at the line in declaration order (those not yet in scope are absent), with the caught exception variable; then for object nodes their fields. Row: icon, name, `= `, value text (`{Type@id}` label rules in 5.1), grey type text when `Show type` is on. Nodes are expandable; loading shows `Collecting data...`; setting a value shows `Modifying value...`; evaluating `Evaluating...`; failures `Cannot load value: {0}`. Lists longer than 100 children show the first 100 then `... (N more items. Double-click to see)` (unknown count: `... (Double-click to see more items)`). New local variables of the line are auto-scrolled into view (`Autoscroll to new local variables`, default on). | Case EXE-DBG-070 case 1..2. |
| EXE-DBG-071 | MUST | Value presentation (classic Java renderers): `{java.lang.String@123} "text"` style: object label shows the type short or qualified name per setting and the object id `@N` when `Object id` is on (default on); declared type off; fully qualified names off; hex value off; strings show the type off; collections/arrays show size `size = N` / `{int[3]@456}`; primitives show the value; `null` shown as `null`; boxed types show the value; `toString()` object view is on for classes overriding `toString` per defaults (`Enable 'toString()' object view` + `For all classes that override 'toString()' method`); long strings are truncated with `... (length: N)` and a `View` link (`... View`). | A String shows its quoted value. |
| EXE-DBG-072 | MUST | Variable actions (context menu group `XDebugger.ValueGroup`, in this order): `Inspect...`, `Mark Object...` (shortcut of `Toggle Bookmark`, `F11` on Windows/Linux), `Set Value...` (`F2`), `Copy Value` (`Ctrl+C`), `Compare Value with Clipboard`, `Copy Name`, separator, `Evaluate Expression...`, `Evaluate In Console`, `Add to Watches`, `Show Referring Objects...`, separator, `Jump To Source` (`F4`), `Jump To Type Source` (`Shift+F4`), separator, `Pin to Top`/`Unpin`; Java adds `Show types`, `View Text`, `Adjust Range...`, `Filter...`, `Create...`/`Customize Data Views...`, `Mute Renderers`, `View as` submenu. | Right click on a variable shows the list in that order. |
| EXE-DBG-073 | MUST | `Set Value` (`F2`) edits the value inline: primitives and strings accept literals, objects accept any expression of the target type; Enter applies (`Set`), Esc cancels (`Cancel`); an error shows the dialog titled `Error Setting the Value`; the tree node refreshes and, for locals, the new value is also shown inline in the editor. | Set `x` from 1 to 5: the node shows `x = 5`. |
| EXE-DBG-074 | MUST | `Sort Variables Alphabetically` (default off; gear menu, setting `Sort values alphabetically`); `Show Variable Values in Editor` (`Show values inline`, default on); `Open Files in Preview Tab`; `Enable auto expressions in Variables view` (default on). | Sort on: variables sorted by name. |
| EXE-DBG-075 | MUST | **Inline values**: with `Show values inline` on, after each variable use or declaration on visible lines up to the execution point the editor shows grey inlay text `name = value` (and the evaluated texts of fields/array elements when cheap); values that changed since the previous stop are drawn highlighted (`Inlined modified values`); the execution line uses its own style (`Inlined values for execution line`). Clicking an inlay selects the variable in the tree. Inlays disappear when the session resumes (and are rebuilt at the next stop). Values never exceed a length limit (truncated with `...`). | Case EXE-DBG-075 case 1. |
| EXE-DBG-076 | MUST | **Watches** pane: empty text `No watches`; toolbar `New Watch...` (`Insert`), `Remove Watch` (`Delete`), `Move Watch Up` / `Move Watch Down`, `Duplicate Watch` (`Ctrl+D`); also `Edit...` (`F2`), `Remove All Watches`, `Pause Watch`/`Resume Watch` (paused watch shows `The watch is paused`, value `... Evaluate`). Adding opens an inline editor with history; a watch is re-evaluated at every stop in the selected frame; errors render in red with the message; watch expressions persist per project (`.idea/workspace.xml` `XDebuggerManager` `watches-manager`). Drop text `Add to watches` when dragging a variable. | Case EXE-DBG-076 case 1. |
| EXE-DBG-077 | SHOULD | `Add to Watches` (editor context menu `Debug` group) adds the selected text or expression at the caret; `Add Inline Watch` (adds an expression shown inline in the editor at the caret, group `Inline Watches`, `Edit Watch Expression`); `Evaluate in Console`. | Select `a+b`, `Add to Watches`: watch row `a+b`. |
| EXE-DBG-078 | SHOULD | `Separate Watches` splits Watches and Variables panes; `Pin to Top`; `Mark Object` assigns a label (dialog `Select Object Label`, `Label:`, `Preview:`, color; duplicate warning `Label is already in use and will be replaced`) shown as `name` on the value in every view for that object while the session lives. | Marked objects show the label. |

### 5.1 Java renderers (observable)

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-DBG-080 | MUST | Settings page `Data Views` (Java) defaults: `Show hex value for primitives` off; `Show type for strings` off; `Enable alternative view for Collections classes` off; `Autoscroll to new local variables` on; `Hide null elements in arrays and collections` on; `Synthetic fields` on; `$val fields as local variables` on; `Static fields` off; `Static final fields` off; `Declared type` off; `Fully qualified names` off; `Object id` on; `Auto populate Throwable object's stack trace` on; data-flow assist `Predict condition values and exceptions based on data flow analysis` on, `Gray out blocks of code that are predicted to be unreachable` on; array range `Array start index` 0, `end index` unlimited, entries limit 100. | The values are the initial state of the page. |
| EXE-DBG-081 | SHOULD | Custom type renderers (`Customize Data Views...`: `Renderer name:`, `Apply renderer to objects of type (fully qualified name):`, `When rendering a node` / `When expanding a node` with `Use default renderer`, `Use following expression:` or `Use list of expressions:`, `Test if a node can be expanded (optional):`, `Append default children`, on-demand); collections/map entry views; array chunking by 100 with `[0..99]` ranges. | A renderer with expression `size()` shows that text. |
| EXE-DBG-082 | LATER | Memory view, stream debugger (`Trace Current Stream Chain`), collection history, `Show Referring Objects` details: listed in the inventory only. | Not applicable. |

## 6. Evaluate Expression

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-DBG-090 | MUST | `Evaluate Expression...` (`Alt+F8`): dialog titled `Evaluate` (title shown as `Evaluate Expression` / `Evaluate Code Fragment`) with the editor `Expression:`, the language hint `Click to change the language`, buttons `Evaluate` (mnemonic `v`) and `Close`, switch buttons `Code Fragment Mode` / `Expression Mode`, result label `Result:` with the value tree (default node name `result`); history navigation hint `Use {0} to navigate through the history`; hint `Use {0} to add to Watches`. The selection or the expression at the caret pre-fills the field. `Enter` evaluates in expression mode (code fragment mode: `Ctrl+Enter`/the button). | Case EXE-DBG-090 case 1..2. |
| EXE-DBG-091 | MUST | The expression is resolved in the scope of the selected frame (locals, fields, `this`, statics, imports of the file; code fragments may declare local variables and contain several statements, last expression value is the result). Side effects execute in the debuggee. Evaluation resumes only the evaluating thread for the duration of the call, so a call that waits for a lock held by another thread can hang until cancelled. A running evaluation shows `Evaluating...` with cancel. | Evaluating `a + b` returns the sum. |
| EXE-DBG-092 | MUST | Evaluation errors use these texts (non-exhaustive): `Cannot evaluate, current stack frame doesn't support evaluation`, `Code fragment expected`, `Evaluation is not supported during another method's evaluation`, `Sources do not correspond to code being executed`, `Stack frame unavailable`, `Object has been collected`, `Thread has been resumed`, `Invalid expression : {0}`, `Cannot find local variable '{0}'`, `Variable '{0}' is not declared`, `Type mismatch`, `Method threw '{0}' exception.`, `Cannot evaluate: process exited`, `Evaluation is not possible during the project analysis`, `Lambdas evaluation is not supported`, `Class '{0}' is not loaded`. | Case EXE-DBG-092 case 1. |
| EXE-DBG-093 | MUST | `Quick Evaluate Expression` (`Ctrl+Alt+F8` and `Alt+click`) evaluates the selected text or expression under the caret in a popup tree with the value; `Esc` closes. Value tooltips on hover (`Show value tooltip.`, default on; delay `Value tooltip delay (ms):` 700; `Show value tooltip on code selection`) show the same tree for the hovered expression only while paused; with the setting off use `Quick Evaluate Expression`. | Hover over `x` after 700 ms shows its value. |
| EXE-DBG-094 | SHOULD | Dialog remembers recent expressions (limit advanced setting `Maximum number of recent expressions`), the mode and the last used language; `Show Evaluate Expression Field` toggles an evaluate field above the variables. | History keeps previous expressions. |

## 7. Settings and defaults (debugger pages)

| Setting | Default |
|---|---|
| `Show debug window on breakpoint` | on |
| `Hide debug window on process termination` | off |
| `Focus application on breakpoint` | per platform (off) |
| `Scroll execution point to center` | off |
| `Show values inline` | on |
| `Show value tooltip` / delay | on / 700 ms |
| `Sort values alphabetically` | off |
| `Confirm removal of conditional or logging breakpoints` | off |
| `Click line number to perform run to cursor` | on |
| `Unmute Breakpoints on Session Finish` | off |
| Transport (Debugger / Java) | `Socket` |
| `Show alternative source switcher` | on |
| `Attach memory agent` | per platform |
| `Resume only current thread` | off |
| `Kill process immediately` | off |
| HotSwap: `Reload classes after compilation` | `Ask` |
| HotSwap: `Build project before reloading classes` | on |
| HotSwap: `Enable 'JVM will hang' warning` | off |
| HotSwap: `Suggest HotSwap in the editor when code is modified` | on |
| `Hide stack frames using stepping filters` | on |
| `Evaluate finally blocks on pop frame and early return` | `Ask` |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-DBG-100 | MUST | These settings exist with these defaults, persisted in `debugger.xml` (IDE level), and take effect for new stops without restarting. | Changing `Show values inline` hides inlays at the next stop. |

## 8. Actions, menus and shortcuts (debugger)

`Run | Debugging Actions` (popup in the Run menu, text `Debugging Actions`) lists in order: the HotSwap
group (`Update Running Application`, `Compile and Reload Modified Files` when applicable), separator,
`Step Over`, `Force Step Over`, `Step Into`, `Force Step Into`, `Smart Step Into`, `Step Out`, `Run to
Cursor`, `Force Run to Cursor`, `Reset Frame`, `Pause Program`, `Resume Program`, separator, `Evaluate
Expression...`, `Show Execution Point`; `Toggle Breakpoint` popup: `Restore Breakpoint`, `Line
Breakpoint`, `Temporary Line Breakpoint`, `Toggle Breakpoint Enabled`; then `View Breakpoints...`.

| Action (text) | Id | Shortcut |
|---|---|---|
| Step Over | `StepOver` | `F8` |
| Step Into | `StepInto` | `F7` |
| Smart Step Into | `SmartStepInto` | `Shift+F7` |
| Step Out | `StepOut` | `Shift+F8` |
| Force Step Over | `ForceStepOver` | `Alt+Shift+F8` |
| Force Step Into | `ForceStepInto` | `Alt+Shift+F7` |
| Run to Cursor | `RunToCursor` | `Alt+F9` |
| Force Run to Cursor | `ForceRunToCursor` | `Ctrl+Alt+F9` |
| Resume Program | `Resume` | `F9` |
| Pause Program | `Pause` | none |
| Show Execution Point | `ShowExecutionPoint` | `Alt+F10` |
| Evaluate Expression... | `EvaluateExpression` | `Alt+F8` |
| Quick Evaluate Expression | `QuickEvaluateExpression` | `Ctrl+Alt+F8`, `Alt+click` |
| Toggle Line Breakpoint | `ToggleLineBreakpoint` | `Ctrl+F8` |
| Toggle Temporary Line Breakpoint | `ToggleTemporaryLineBreakpoint` | `Ctrl+Shift+Alt+F8` |
| Toggle Breakpoint Enabled | `ToggleBreakpointEnabled` | none |
| View Breakpoints... | `ViewBreakpoints` | `Ctrl+Shift+F8` |
| Edit Breakpoint | `EditBreakpoint` | `Ctrl+Shift+F8` (when on a breakpoint line) |
| Reset Frame | `Debugger.PopFrame` | none |
| Set Value... | `XDebugger.SetValue` | `F2` |
| New Watch... | `XDebugger.NewWatch` | `Insert` |
| Jump To Type Source | `XDebugger.JumpToTypeSource` | `Shift+F4` |
| Attach to Process... | `XDebugger.AttachToProcess` | `Ctrl+Alt+F5` |
| Mark Object... | `Debugger.MarkObject` | as `Toggle Bookmark` |
| Mute Breakpoints | `XDebugger.MuteBreakpoints` | none |
| Activate Debug Tool Window | `ActivateDebugToolWindow` | `Alt+5` |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-DBG-101 | MUST | The menu structure, the texts and shortcuts above are implemented; stepping/resume/pause actions are enabled only in a session of the right state (steps only when paused; `Pause` only when running; `Resume` when paused). Shortcuts of macOS follow the macOS keymap (not listed in the source for debug keys: they equal the Windows keys with Cmd/Alt substitutions in the macOS keymap file; verify). | In a running (not paused) session Step Over is disabled. |
| EXE-DBG-102 | MUST | `Attach to Process...` (`Ctrl+Alt+F5`): popup `Attach To` listing local processes (columns `Executable`, `Process ID`, `User`, `Command Line`; `Available Debuggers`; `Show Only My Processes`; empty `No processes to attach to`; error `Failed to get processes`); choosing a Java process starts a debug session attached to it (`Attach with {0}`). `Attach Debugger` in the Run window attaches to a run configuration process. | Selecting a JVM attaches and breakpoints work. |
| EXE-DBG-103 | MUST | Toolbar buttons of the Debug window top toolbar (left to right): `Rerun`, `Stop`, separator, `Resume Program`, `Pause Program`, `Step Over`, `Step Into`, `Step Out`, separator, `View Breakpoints`, `Mute Breakpoints`; the `More` popup: `Force Step Over`, `Force Step Into`, `Smart Step Into`, separator, `Run to Cursor`, `Force Run to Cursor`, separator, `Show Execution Point`, separator, `Evaluate Expression`, `Reset Frame`. | The toolbar shows the 11 entries. |
| EXE-DBG-104 | SHOULD | Customization of the debugger toolbar (`Add Action to Debugger Toolbar`, `Customize Debugger Toolbar`) and the layout settings (`Restore Default Layout`, `Show Tab Labels`). | Not applicable. |

## 9. HotSwap

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-DBG-105 | SHOULD | After editing and compiling classes in a running Java debug session, the IDE offers to reload the changed classes: with `Reload classes after compilation` `Ask` the dialog titled `Reload Changed Classes` (`Reload Changed Classes for {0}`) says `Some classes have been changed. Reload changed classes now?` with `Reload` / `Skip`; `Always` reloads silently; `Never` does nothing. The editor shows the HotSwap toolbar (`Code changed:` and a reload button; setting `Suggest HotSwap in the editor`). Progress: `Scanning for classes to reload...`, `Reloading classes...`; results: `Code has been reloaded` / `N classes reloaded` / `Loaded classes are up to date. Nothing to reload`; failures `HotSwap failed` with `Restart session` / `Stop session`; unsupported changes `HotSwap is unlikely to apply these changes`. | Change a method body, press the reload button: next call uses the new body. |
| EXE-DBG-106 | SHOULD | Actions: `Compile and Reload Modified Files`, `Reload Changed Classes`, `Compile And Reload File` (description `Compile the file and HotSwap all changed classes`). | Listed in the menu when a session is active. |

## 10. Java debugger specifics (observable contract over JDWP)

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-DBG-110 | MUST | Launch for Debug adds to the JVM command line the JDWP agent: `-agentlib:jdwp=transport=dt_socket,address=<host>:<port>,suspend=y,server=n` (JDK 5+ style) with a free local port; the IDE listens before the VM starts (status `Listening to the connection, address: '...'`; then `Connected to the target VM, address: '...', transport: 'socket'` in the console). Shared-memory transport (`shared memory`) only on Windows; setting `Transport: Socket`. | A debug run prints the Connected line. |
| EXE-DBG-111 | MUST | Remote JVM Debug: `Attach to remote JVM` connects to `host:port` of a JVM started with `-agentlib:jdwp=...server=y`; `Listen to remote JVM` waits for a VM to connect (status `Debugger is waiting for application to start`). Failure messages: `Failed to establish connection to the target VM, address: {0}`, `Unable to open debugger port`; JRE mismatch: `Target JRE version ({0}) does not match project JDK version ({1}), will use sources from JDK: {2}`. Stop in attach mode disconnects without killing the VM. | Attach to a `server=y` JVM works. |
| EXE-DBG-112 | MUST | Stop texts in the status area: `Breakpoint reached at {Class}.{method}({File}:{line})` for line breakpoints; `Stopped at cursor` for run to cursor; `Process started`, `Process terminated`, `The application is running`, `Stepping in the application`, `Paused`, `Debug stopped`, `Waiting until last debugger command completes`; `Disconnected from the target VM, address: {0}`. | After hitting a breakpoint the status shows the first. |
| EXE-DBG-113 | MUST | Line breakpoints resolve through class-prepare events: a breakpoint in a class not yet loaded is armed (pending) and becomes active when the class is prepared; the same for lambdas and inner/anonymous classes of the file (all classes whose source file and line match). Breakpoints in generated/inline code (Kotlin inline functions) are LATER. | Breakpoint in a not yet loaded class stops after loading. |
| EXE-DBG-114 | MUST | Source mapping: a stop location (class + source name + line) is mapped to a source file: first by the class's qualified name in the project/library sources, then by file name within the package; if several sources match, the `alternative source switcher` is shown (default on). Decompiled/sources-less classes show the decompiled view (see Java spec). | A stop inside a library class opens its source or decompiled code. |
| EXE-DBG-115 | SHOULD | Async stack traces (capture points) are shown below the real frames with the header `Async call from {0}` in a grey frame when available (action `Async Stack Traces`). | Not applicable without the agent. |
| EXE-DBG-116 | LATER | `Debug Build Process`, `Throw Debug Exception`, `Repair Debugger View`, Kotlin coroutine debugger, `Switch to the Next Stopped Breakpoint` across VMs, memory agent and stream debugger. | Not applicable. |

## 11. Open questions

1. Default shortcut and the placement of the global `Next Frame`/`Previous Frame` actions (hint
   `Switch frames from anywhere in the IDE with {0} and {1}`): not in the default keymap file; the
   source registers them as list actions of the frames pane.
2. macOS keymap values of the debug actions (the macOS keymap file overrides some; the Windows/Linux
   values are authoritative here).
3. Whether the Java debugger listens (`server=n`) in all configurations or only with newer JDKs; the
   source supports both listening and attach modes and the status texts of both exist.
4. Behavior of the exact default for `Focus application on breakpoint` (not found in the sources read).

# 03. Run tool window and console

Status: specification. Requirement prefix `EXE-CON`. Cases: `cases/03-run-tool-window-and-console.cases.md`.
Launch and Stop semantics are in `02-run-from-context.md`. Editor foldings/soft wraps use the
editor specs; the console is an editor in read-mostly mode.

ic: `platform/consoleView/impl/src/com/intellij/execution/impl/{ConsoleViewImpl.kt,ConsoleBuffer.java,EditorHyperlinkSupport.java,ConsoleViewTypedHandler.kt}`,
`platform/consoleView/src/com/intellij/execution/{ConsoleFolding.java,filters/*}`,
`platform/consoleView/resources/messages/ConsoleViewBundle.properties`,
`platform/ide-core/src/com/intellij/execution/process/ProcessTerminatedListener.java`,
`platform/platform-util-io/src/com/intellij/execution/process/{AnsiEscapeDecoder,ColoredProcessHandler}.java`,
`java/execution/impl/src/com/intellij/execution/filters/{StackTraceFolding,StackTraceFoldingSettings}.kt`,
`java/execution/openapi/src/com/intellij/execution/filters/{ExceptionFilter,ExceptionWorker}.java`,
`platform/execution-impl/resources/intellij.platform.execution.impl.actions.xml` (Runner.* actions),
`platform/execution/resources/messages/ExecutionBundle.properties`.

## 1. Tool window and tabs

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-CON-001 | MUST | Tool window `Run` (id `Run`, shortcut `Alt+4`) holds one **tab per run content** (configuration run); the tab title is the configuration name; the icon shows running/finished state. The Debug executor uses the tool window `Debug` (`Alt+5`) with the tabs `Debugger` and `Console`. | Running `Main` opens the Run window with tab `Main`. |
| EXE-CON-002 | MUST | When a run starts the tool window is activated unless the configuration has `Open run/debug tool window when started` off (default on); it takes keyboard focus only when `Focus run/debug tool window when started` is on (default off). A failed launch does not open the window. | Default run: window shown, editor keeps focus. |
| EXE-CON-003 | MUST | A new run of a configuration whose previous tab is finished reuses that tab (console cleared); a tab whose process still runs is left alone and a second tab is used when parallel runs are allowed. | Case EXE-CON-003 case 1. |
| EXE-CON-004 | MUST | Left toolbar of a run tab (top to bottom): `Rerun` (`Ctrl+F5`), `Stop` (`Ctrl+F2`), then the executor-specific actions (Java: `Dump Threads`, `Exit`), then console actions in the order: `Up the Stack Trace`, `Down the Stack Trace`, `Use Soft Wraps`, `Scroll to the end`, `Print`, `Clear All`; then tool-window options; the context menu of the console contains `Fold Lines Like This`, `Pause Output`, `Clear All`, `Copy`. | Case EXE-CON-004 case 1. |
| EXE-CON-005 | MUST | Tab context menu: `Close` (`Runner.CloseView`), `Close Others`, `Close All`, `Close All but Pinned`, `Pin Tab`/`Unpin Tab`; layout menu: `Show Tab Labels`, `Restore Default Layout`, `Focus on Startup`. Closing a tab whose process runs first asks to stop it (see open question 1); closing a finished tab just removes it. | `Close All but Pinned` keeps pinned tabs. |
| EXE-CON-006 | SHOULD | Select next/previous tab: `Select Next Tab`, `Select Previous Tab`; `Show List of Tabs`. | Alt+Right switches tabs in the Run window. |
| EXE-CON-007 | LATER | Services tool window (`Alt+8`) hosting run configurations (`Group Configurations`, `Hide Configuration`, ...) is not specified; the classic Run/Debug windows are the default. | Not applicable. |

## 2. Console output

The console shows the process output and interleaves a few IDE-written lines. Output types:
*standard output* (default text color), *standard error* (red), *system* (grey/blue: command line,
exit message), *user input* (green).

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-CON-010 | MUST | First line of a Java run: the command line (system output, e.g. `/path/to/java -classpath ... p.Main`) is printed; Debug additionally prints `Connected to the target VM, address: 'host:port', transport: 'socket'`. The command line is shown folded when long (soft-wrapped single logical line may fold). | Case EXE-CON-010 case 1. |
| EXE-CON-011 | MUST | When the process ends the console appends an empty line, `Process finished with exit code N` (system output) and a newline. `N` is the numeric exit code; on Unix a code above 128 that is a signal exit appends ` (interrupted by signal S:NAME)` (for example `130 (interrupted by signal 2:SIGINT)`), on Windows codes in `0xC0000000`..`0xCFFFFFFF` append ` (0xHEX)` and `0xC000013A` appends `: interrupted by Ctrl+C`. The same text is set in the status bar. A debug session additionally prints `Disconnected from the target VM, address: 'host:port', transport: 'socket'` before it. | Exit 0: `Process finished with exit code 0`. |
| EXE-CON-012 | MUST | Output is buffered and flushed to the editor every 200 ms (a request after a longer pause flushes immediately); order of stdout and stderr chunks is preserved as received. A single line is never split by the flush. | 1000 quick `println` calls show in order with at most a 200 ms delay. |
| EXE-CON-013 | MUST | Cycle buffer: the console keeps the last 1024 KB of text by default (`idea.cycle.buffer.size` KB; a UI setting can override); older text is dropped from the beginning when exceeded, whole lines at a time, and the hyperlinks/folding of removed text vanish. A value of `disabled` removes the limit. | Printing 3 MB leaves the last ~1 MB. |
| EXE-CON-014 | MUST | Carriage returns: `\r` without `\n` moves the write position to the line start so later text overwrites; `\r\n` is a normal line end. Tabs render with the editor's tab stops (console default 8). | `abc\rX` renders `Xbc`. |
| EXE-CON-015 | SHOULD | Very long lines are cut for display (the editor's soft-wrap applies; extremely long output lines are truncated with a marker after a platform limit) without affecting the stored text. | A 5 MB single line does not freeze the UI. |
| EXE-CON-016 | MUST | `Clear All` (toolbar and menu; description `Clear the contents of the console`) removes all text, hyperlinks and folds, keeps the process, and resets sticky scrolling. | After clearing, new output starts at line 1. |
| EXE-CON-017 | SHOULD | `Pause Output` (context menu) suspends flushing new text into the console (the process continues; text is queued) until toggled off; the queue is then flushed in order. | While paused no new text; when resumed all appears. |
| EXE-CON-018 | SHOULD | `Print` prints the console text; `Copy Content`, `Copy Selected Content` copy all/selection. | Copy puts plain text on the clipboard. |

### 2.1 ANSI colors

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-CON-020 | SHOULD | Processes started with the colored handler interpret ANSI escape sequences `ESC [ ... m` (SGR): reset (0), bold (1), faint (2), italic (3), underline (4), inverse (7), normal intensity (22), foreground 30-37 and bright 90-97, background 40-47 and 100-107, defaults 39/49, 256-color (`38;5;n`, `48;5;n`) and true-color (`38;2;r;g;b`, `48;2;r;g;b`). The escape characters are removed from the visible text. Unknown sequences are dropped. | Case EXE-CON-020 case 1. |
| EXE-CON-021 | SHOULD | A sequence split across two output chunks is held until complete (no raw `ESC[` fragments shown). Colors persist across lines until reset. | `\u001b[3` and `1mred` in separate chunks render red `red`. |
| EXE-CON-022 | SHOULD | Non-color sequences (cursor movement, erase) are ignored in the console except erase-line which is dropped; a real terminal belongs to the Terminal tool window. | `ESC[2K` shows no artifacts. |

## 3. Hyperlinks

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-CON-030 | MUST | Java stack frames `at pkg.Class.method(File.java:LINE)` get a hyperlink on the `File.java:LINE` part; click (and `Ctrl+click`) opens the source file at that line (column 1). `Native Method` and `Unknown Source` frames are not linked. Frames inside lambdas/inner classes link to the file of the outermost class when the class is found by the qualified name; the file is resolved by class name first and by file name within the stack frame's package otherwise. Not found: no link. | Case EXE-CON-030 case 1..2. |
| EXE-CON-031 | MUST | `Caused by:` and `Suppressed:` blocks and `\t... N more` lines are plain text; their frames are linked as above. The first line `Exception in thread "main" java.lang.X: message` is plain text, the exception class name part is linked to the class source when found. | Class name `java.lang.IllegalStateException` links to its source or decompiled class. |
| EXE-CON-032 | MUST | URL filter: text matching `http://`, `https://`, `ftp://`, `file://` URLs becomes a link that opens in the browser (`file:` opens in the editor); trailing punctuation `.,;)` is not part of the link. | `see https://x.y/z.` links `https://x.y/z`. |
| EXE-CON-033 | MUST | File path filter: `path/File.ext:LINE[:COLUMN]` and `(path/File.ext:LINE)` patterns for files that exist (absolute or relative to the project/working directory) become links opening the file at line and column. | Compiler output `src/Foo.java:12:5: error` links `src/Foo.java:12:5`. |
| EXE-CON-034 | MUST | Hyperlinks are computed incrementally per flushed chunk on a background pass; heavy filters may run after the text is visible. A link hover shows the pointer cursor and, for file links, underlines the text. After `Clear All` there are none. | Link appears within a short time after the line is visible. |
| EXE-CON-035 | SHOULD | `Up the Stack Trace` / `Down the Stack Trace` (previous/next occurrence) move the caret between hyperlinks/stack frames that are source links, selecting and navigating. | Down moves to the next link. |
| EXE-CON-036 | SHOULD | Hyperlinks with hover popups (e.g. exceptions) show the full `Analyze stack trace`; custom filters contributed by plugins attach by the console filter provider. | Not applicable without plugins. |

## 4. Folding

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-CON-040 | MUST | Java stack trace folding (settings `Java Stack Trace` page: `Fold stack trace longer than` `8` `lines`; default enabled, threshold 8): in a block of consecutive frame lines (`\tat ...`) or `\t... N more` lines, the first 8 stay visible and all following lines fold into one placeholder `\t<K folded frames>` (singular `<1 folded frame>`) where `K` is the number of folded lines; if any folded line is an async-trace separator the text is `\t<K folded frames (including async stack trace)>`. The placeholder is collapsed by default; click expands. A non-frame line resets the count. Folding is disabled in consoles marked "no stack trace folding" (test details). | Case EXE-CON-040 case 1..2. |
| EXE-CON-041 | MUST | The folding placeholder text uses the exact messages `<{0} folded {0,choice,0#frames|1#frame|2#frames}>` and the async variant. | 12 frames: visible 8, placeholder `<4 folded frames>`. |
| EXE-CON-042 | SHOULD | `Fold Lines Like This` (context menu on selected text) creates a user folding rule for lines matching the selected text; user rules persist (console folding settings). | Select `DEBUG`, Fold: lines with `DEBUG` fold. |
| EXE-CON-043 | SHOULD | Plugin foldings (Spring, Gradle, ...) collapse groups of lines with their own placeholders; a folding extension may attach to the previous line. Nested foldings order by priority. | Not applicable without plugins. |

## 5. Input

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-CON-050 | MUST | While the process runs, typing in the console inserts text at the end (user input). `Enter` sends the typed line plus newline to the process's standard input and shows it in the console as user-input text. Typing and `Enter` with a process that has no input stream reports `No process input`. | Case EXE-CON-050 case 1. |
| EXE-CON-051 | MUST | `Send EOF` (`Ctrl+D` in the console) closes the process input; subsequent reads see end of file. | A program blocked on stdin ends after Ctrl+D. |
| EXE-CON-052 | MUST | When the process has finished, typing is ignored (read-only). Paste of multi-line text sends line by line. | Typing after termination changes nothing. |
| EXE-CON-053 | SHOULD | `Redirect input from:` a file (EXE-CONF-084) feeds the file content and the console shows no user input. | Not applicable. |

## 6. Scrolling and soft wrap

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-CON-060 | MUST | **Stick to end**: while the caret/scroll is at the end, new output scrolls the view; scrolling up manually (or clicking inside the text) stops following; `Scroll to the end` (toolbar toggle) re-enables following and jumps to the end; the selection, if any, is preserved. | Case EXE-CON-060 case 1. |
| EXE-CON-061 | MUST | `Use Soft Wraps` toggle wraps long lines at the viewport width; default off; the setting is remembered per console place. | Toggle changes layout only. |
| EXE-CON-062 | SHOULD | Large outputs keep the UI responsive: flushing is batched (EXE-CON-012) and the view never scrolls to the end repeatedly within one flush. | 100k lines scroll without lag. |

## 7. Debug-specific console wiring

The Debug window's `Console` tab is the same console (all above applies). The `Debugger` tab is in
`05-debugger-core.md`. When a debug session ends the tab remains, console intact.

## 8. Open questions

1. Text of the confirmation shown when closing a tab of a running process (not in the read bundles);
   Archipel should show `Stop` and `Cancel` choices.
2. Default console font and colors follow the editor color scheme console keys (`Console` group in Colors & Fonts); the exact key names are
   specified by the color-scheme spec, not here.
3. Truncation limit for very long lines (platform constant).

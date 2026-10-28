# TUI QA bug list (archipel-ide --tui, debug build of main at 0.2.6)
Test env: tmux 3.4 (legacy mode; "extended" = tmux `extended-keys always`, tmux 3.4 has no csi-u format, so the app got xterm modifyOtherKeys-style keys), xterm 9pt on Xvfb :78. Projects in qa/demo (Java Maven), qa/rust, qa/txt.


## Works (verified)
Startup with/without project/file, dashboard, Open Folder dialog; resize 1x1..250x70 without panic; normal quit restores terminal and saves all; typing, Enter/indent, Backspace/Delete, word/line movement, selection, undo/redo, Ctrl+D/Y, Alt+Shift+Up/Down, Alt+J, comment (Ctrl+/ and Ctrl+7), Alt+Up extend, copy/cut/paste incl. OSC52, bracketed paste (<=6k lines), CRLF/BOM preserved when unchanged, 50k-line file fast, kitty-encoded keys (CSI u incl. double Shift) and legacy keys for the whole keys table, chords, popups (Search Everywhere tabs, Go to File, Find Action, Recent Files, Go to Line, Find/Replace bar, Find in Files, completion, Alt+Enter) with no leftovers after Esc, Back/Forward, Go to Declaration, F2 problems, tree open/new file/delete/speed search, tab switching/closing, splitter drag, wheel scroll, status segment clicks (run), themes cycle, Commit window (commit, auto refresh), Terminal (colors, UTF-8, resize, Ctrl keys), Run (shell/cargo, stop, rerun), save on focus loss, external reload, deleted file closes tab, folding via chord and placeholder click.

### TUI-BUG-001 Ctrl+K chord times out after ~2 s; the which-key list disappears
severity: major
area: keys / chords
steps: legacy: press Ctrl+K, wait ~2.5 s (list is shown after 0.3 s), then press `n`.
expected: docs/tui.md "A chord shows a which-key list after its first stroke: press Ctrl+K and wait to see the fallbacks." The chord should stay pending until a key or Esc.
actual: after ~2 s the which-key list vanishes and the chord is cancelled; `n` is then handled by the focused widget (typed into the project tree's speed search, or inserted into the document). A user who reads the list cannot use it. Measured: works with 1.5 s delay, fails with 2.5 s.
terminal mode: legacy and extended

### TUI-BUG-005 Save/quit on a file that cannot be saved: message cut off, quit discards edits silently
severity: major
area: save / quit / external change
steps: 1. open e.txt, type text (unsaved). 2. change e.txt on disk from another process. 3. Ctrl+S. 4. Ctrl+K Q.  (also reproduced with a chattr +i read-only file)
expected: IntelliJ: conflict dialog (keep memory / reload disk); read-only files cannot be typed into; quit must not drop unsaved work silently. docs/tui.md: "Quit (saves everything first)".
actual: Ctrl+S: status line "command failed: Not saved, changed on d…" (cut off, full text only in a toast: "(load the disk version or kee" also cut); for the immutable file "command failed: ro.txt: filesystem error: Operation not permitted…". Ctrl+K Q then exits with code 0 immediately: no warning, edits are lost, and nothing is said. Read-only files accept typing (no lock indicator, no refusal) and the failure only shows at save time.
terminal mode: legacy

### TUI-BUG-007 Run window: relative source locations are not clickable / Enter does nothing
severity: major
area: Run window
steps: Rust project with an error (println!("{}", y)); Shift+F10; Alt+4; Up/Down to the line ` --> src/main.rs:3:26`; Enter (also try on `txt/e.txt:2`).
expected: docs/tui.md "a line with a source location opens the file (Enter or double click)". rustc/cargo/javac/maven print project-relative paths.
actual: nothing happens (caret stays Ln 1, Col 1, no message). An absolute path `/…/txt/e.txt:3:4: error: bad` works (jumps to Ln 3, Col 4). Relative paths are never resolved.
terminal mode: legacy

### TUI-BUG-008 "Choose run configuration" lists nothing for detected Cargo/Maven targets
severity: major
area: Run
steps: open the Cargo project (qa/rust) or the Maven demo; Alt+Shift+F10.
expected: docs/tui.md "run configurations of the project (.archipel/runConfigurations, detected Cargo, Gradle, Maven, npm and Go targets)"; the chooser lists them.
actual: popup "Run Configurations / No matches". Status segment says "Current File" (Maven demo) / "rust" (cargo) and Shift+F10 works for cargo ("cargo run"); in the Maven demo Shift+F10 gives error toast "command is disabled: run.project" with no explanation (java and mvn are installed). With hand written .archipel/runConfigurations/*.toml the chooser lists them correctly.
terminal mode: legacy

### TUI-BUG-011 Typed/pasted newline in a CRLF file is written as LF: mixed line endings on save
severity: major
area: editor / save
steps: file crlf.txt = "line1\r\nline2\r\n\tindented tab\r\nlast". Open, Ctrl+Home, type AB, Enter, End, Enter, type "new", Ctrl+S.
expected: IntelliJ keeps one separator style per document (CRLF everywhere).
actual: `cat -A`: "AB$ / line1^M$ / new$ / line2^M$ / ^Iindented tab^M$ / last": new lines are bare LF, old ones CRLF.
terminal mode: legacy

### TUI-BUG-014 Closing a tab with unsaved changes silently discards the edits
severity: major
area: tabs / data loss
steps: xterm, open e.txt, edit it (tab shows the modified dot), Ctrl+F4 (or click the tab close).
expected: IntelliJ keeps/saves the document (no loss); at the very least a prompt.
actual: tab closes at once, no prompt, no save; e.txt on disk still has the old content (verified with cat).
terminal mode: legacy

### TUI-BUG-015 Pasting large text freezes the UI (quadratic time)
severity: major
area: editor / paste / performance
steps: tmux `load-buffer` a file of N lines ("pasted line k"), caret in App.java, `tmux paste-buffer -p -r` (bracketed paste).
expected: instant like the GUI.
actual: 2,000 lines (57 KB): 2.6 s; 6,000 lines (170 KB): 13 s; 20,000 lines (570 KB): about 150 s with the UI frozen (no key reacts, 80-200% CPU), undo of that paste again takes minutes. Opening a 50,000 line file is instant (1.5 s), so the cost is in the paste path.
terminal mode: legacy

### TUI-BUG-019 Caret not scrolled into view inside a very long wrapped line
severity: major
area: editor / scrolling
steps: txt/long.txt = 5,000 x characters + " end" on one line; open it; press End; type ZZZ.
expected: view scrolls to the caret.
actual: status says Ln 1, Col 5005 but the view still shows the first 30 rows of the wrapped line (43 rows tall); the typed text is invisible. docs/tui.md acknowledges wrapping, but "caret follows" is expected.
terminal mode: xterm

### TUI-BUG-026 Stop leaves child processes running (orphans)
severity: major
area: Run
steps: config `sh -c "echo slow-started; sleep 30; echo slow-done"`; run; Ctrl+F2 (Stop); `ps`.
expected: IntelliJ kills the whole process tree.
actual: window says "Process stopped"/"Task cancelled" but `sleep 30` keeps running with ppid 1 (verified twice; Rerun then starts a second one). Same risk for mvn/gradle/cargo children.
terminal mode: legacy

### TUI-BUG-027 Conflict after external change has no resolution path
severity: major
area: save / external change
steps: edit App.java (unsaved), change it on disk, Ctrl+S -> "Not saved, changed on disk: .../App.java (load the disk version or keep yours)"; try Ctrl+S again, Find Action "Reload All from Disk" (Enter), Ctrl+Alt+Y.
expected: dialog to keep/reload. 
actual: message only (the status line cuts it after "changed on d…"; readable only at >=250 columns), nothing offers either choice; Reload All from Disk leaves the dirty document and its edits in place; Ctrl+S keeps refusing. Only escape: close the tab (loses edits, BUG-014).
terminal mode: legacy/extended

### TUI-BUG-030 Java code intelligence: completion works only on demand, auto-popup missing, results unranked
severity: major
area: Java code intelligence
setup: Maven project qa/mvn, pom with org.junit.jupiter:junit-jupiter 5.11.0 (test), src/test/java/demo/CalcTest.java; opened with `archipel-ide --tui mvn src/test/java/demo/CalcTest.java`.
steps+actual:
 1. Maven import finished ~15 s after start ("Project imported: 1 module, 8 libraries"). Status LSP segment shows `LSP ~ Java` (tilde = not ready) for the whole session (>2 min); earlier in the demo project Go to Declaration said "command failed: jdtls is still starting". Diagnostics are nevertheless JDT-style text (e.g. "List cannot be resolved to a type"), so a JDT-based source does answer, but IntelliJ wording ("Cannot resolve symbol 'Bar'") is absent.
 2. `    @Te` typed in the class body: NO auto-popup (waited 3 s). Ctrl+Space then offers, in this order: TempDir, Temperature, Test, Testable, TestClassOrder, TestFactory, TestInstance, TestMethodOrder, TestTemplate (junit Test is third, "Temperature" is a non-JUnit class). Choosing Test adds `import org.junit.jupiter.api.Test;` correctly (works).
 3. `List<String> l = new ArrayLi`: no auto-popup; Ctrl+Space lists constructors `ArrayList ()`, `ArrayList (int initialCapacity)`, `ArrayList (Collection<? extends E>)`; accepting adds `import java.util.ArrayList;` (works). `Assertions.assertEq` + Ctrl+Space lists all 20+ assertEquals overloads first (no dedupe/ranking, no unimported-class import).
 4. Earlier in the demo project, auto-popup DID open while typing `Sys` / `Strin` (list of Java classes C System, SystemColor, ...), so auto-popup is flaky: it appeared for identifiers at line start and not for `@Te` / after `new `.
 5. Diagnostics: without saving, `foo();` and `Bar b;` were flagged within ~5 s (Problems: "Bar cannot be resolved to a type 13:9", "The method foo() of type new ArrayList<Object>(){} must override or implement a supertype method 12:9", "This method requires a body instead of a semicolon"), count in status line rose 2 -> 10. Cascading JDT syntax errors appear because completion left `new ArrayList<>()` without `;`. Missing: no "Cannot resolve method 'foo' in 'CalcTest'".
 6. Alt+Enter on `Assertions` (unimported): list is alphabetical JDT code actions, starts with "Add all missing imports", then "Change to 'APOptions' (sun.security.krb5.internal)", 'Assert' (com.sun.tools.javac.util), 'AssertionError'... the real "Import 'Assertions' (org.junit.jupiter.api)" is 11th. On `List`: first items are "Change to 'LCONST' (com.sun.org.apache.bcel.internal.generic)", 'Line' (javax.sound.sampled), 'Link'... JDK-internal packages (sun.*, com.sun.*, jdk.internal.*) are offered; "Import 'List' (java.util)" not visible in the first 9 rows. On `Bar b;`: "Add type parameter 'Bar' to 'CalcTest'" is first, "Create class 'Bar'" 5th.
 Root-cause hints: jdtls is the source (JDT messages, "Change to ..." code actions) but its readiness segment never leaves `~`; code actions and completion are passed through unfiltered/unranked (no IntelliJ-style ordering, internal JDK packages not excluded, no auto-popup trigger on `@`/`new `); the owner's "only String operations" likely comes from jdtls not yet having the classpath/indexed (demo project without Maven deps import, jdtls "still starting") so only the JDK/word items appear.
terminal mode: legacy

### TUI-BUG-002 Ctrl+K chord keys are case sensitive; Shift+letter leaks into the focused widget
severity: minor
area: keys / chords
steps: Ctrl+K, then `N` (capital, e.g. Caps Lock on) with focus in the editor or tree.
expected: Ctrl+K then N (which-key list prints the letters in upper case) opens Go to File.
actual: chord aborted, `N` typed into the tree speed search ("demo     N" shown on the root row) or inserted in the document.
terminal mode: legacy

### TUI-BUG-003 Ctrl+S on a document without changes shows a red error toast
severity: minor
area: save
steps: open any file (xterm/legacy), do not edit, press Ctrl+S.
expected: IntelliJ: Save All does nothing silently. docs/tui.md: "Save All (Ctrl+S)".
actual: red toast "editor.saveAll / command is disabled: editor.saveAll" and the same text in the status line.
terminal mode: legacy

### TUI-BUG-004 Disabled actions are reported as red error toasts with internal command ids
severity: minor
area: feedback / toasts
steps: (a) Ctrl+Z with nothing to undo; (b) Ctrl+Alt+Left with empty navigation history; (c) Alt+Left with a single tab; (d) Ctrl+F2 / Ctrl+F5 with nothing running; (e) Ctrl+K Z with nothing to redo; (f) F2 without problems.
expected: IntelliJ: the action is disabled and silent (or a short hint).
actual: error toasts titled with the command id and "command is disabled: editor.undo", "navigate.back", "editor.previousTab", "run.cancel", "run.rerun", "editor.redo"; toasts pile up in the top right corner and cover the editor ("+1 more").
terminal mode: both

### TUI-BUG-009 Run window does not follow the output; shows the top of long output
severity: minor
area: Run window
steps: config `seq 1 3000; echo END`; run it (Alt+Shift+F10, choose); window shows lines 1..10.
expected: IntelliJ console auto-scrolls to the end while output is produced.
actual: view stays at the first lines; End key is needed.
terminal mode: legacy

### TUI-BUG-010 Run console strips ANSI colors; stderr not distinguished; status mode label says TERMINAL for Run
severity: minor
area: Run window
steps: config `printf "\033[31mred\033[0m\n"; echo err >&2`; run.
expected: IntelliJ console renders ANSI colors and stderr in red.
actual: plain text for both. Also with focus in the Run window (Alt+4) the status-line mode label reads "TERMINAL".
terminal mode: legacy

### TUI-BUG-012 No fold chevrons/markers in the gutter (also not for a folded region)
severity: minor
area: folding / rendering
steps: open App.java, look at lines 5, 8, 13; Ctrl+K - folds the caret block to `{…}`; hover/click the gutter.
expected: docs/tui.md "a fold chevron editor.toggleFold"; IntelliJ shows fold markers in the gutter.
actual: no chevron is ever drawn, clicking the gutter does nothing; only clicking the `{…}` placeholder (works) or a chord toggles the fold.
terminal mode: xterm

### TUI-BUG-013 Non-UTF-8 and binary files cannot be opened (opaque error)
severity: minor
area: robustness
steps: Ctrl+K N, open latin1.txt (bytes "caf\xe9 ...") or a 3 kB random binary.
expected: IntelliJ opens text with a detected/selected charset and shows a "binary file" placeholder for binaries.
actual: toast "editor.openFile / command failed: file is not valid UTF-8: /full/path" (cut at the toast edge); nothing opened. No crash.
terminal mode: legacy

### TUI-BUG-016 Go to Line from the tree moves the caret but leaves keyboard focus in the project tree
severity: minor
area: focus
steps: Alt+1 (tree focus, mode NORMAL), Ctrl+G, `9`, Enter, then type `zzz`.
expected: IntelliJ moves focus to the editor; typed text goes into the document.
actual: caret moves to Ln 9, status mode stays NORMAL, `zzz` goes into the tree speed search ("demo   zzz"). (Go to File and Search Everywhere do focus the editor.)
terminal mode: legacy

### TUI-BUG-017 Stale problems for a deleted file stay in the Problems window
severity: minor
area: Problems
steps: Find Action `New File` -> src/Foo.java (shows a warning for it), delete it via the tree (Delete, Enter), Alt+6.
expected: file's problems disappear.
actual: Problems still lists "Foo.java  src · 1 problem / Foo.java is not on the classpath of project demo, only syntax errors are reported 1:1" although the file does not exist (ls shows it gone).
terminal mode: xterm

### TUI-BUG-018 Navigating to a symbol puts the caret at the start of the line, not on the symbol
severity: minor
area: navigation
steps: Ctrl+K E, type `main` (or `greet`, `names`), Enter on the Symbols/All result.
expected: IntelliJ: caret on the identifier (Ln 8, Col 24 for main).
actual: Ln 8, Col 1 / Ln 13, Col 1 / Ln 6, Col 1. (Text results do land on the match: Ln 15, Col 38.) Go to Declaration (Ctrl+B) lands correctly (Ln 6, Col 32).
terminal mode: legacy

### TUI-BUG-020 Several Ctrl+K chord letters are ambiguous / shadowed
severity: minor
area: keys
steps: Ctrl+K then `e` / `c` / `[` / `]`; which-key list shows two actions for each (E: Extend Selection and Search Everywhere; C: Column Selection Mode and Commit; [ ]: Back/Forward and Move Caret to Code Block Start/End).
expected: each chord reaches its action; the list should not offer shadowed entries.
actual: only Search Everywhere / Commit / Back / Forward run; Extend Selection (E), Column Selection Mode (C), Move Caret to Code Block Start/End are unreachable (Alt+Up covers only Extend Selection).
terminal mode: legacy

### TUI-BUG-021 SIGTERM/SIGINT leave the terminal in raw mode / alt screen / mouse tracking
severity: minor
area: robustness / quit
steps: run the TUI under a pty, send SIGTERM (or SIGINT) to the process.
expected: terminal restored (alt screen off, mouse modes 1000/1002/1003/1006 off, raw mode off, bracketed paste/focus off, kitty flags popped); unsaved docs saved.
actual: process dies with the signal and writes nothing after it (0 bytes), so the terminal stays in the alt screen with any-motion mouse tracking on. (A normal Ctrl+K Q restores everything. SIGHUP alone is ignored while the pty is open; closing the pty ends the process after 1 s.)
terminal mode: n/a

### TUI-BUG-022 Terminal window: Ctrl+K is a chord leader, and a dead shell gives no indication
severity: minor
area: Terminal
steps: (a) Alt+F12, type `abc`, press Ctrl+K, type `a` -> Find Action opens (Ctrl+K never reaches the shell, so readline kill-line is lost). (b) type `exit`, Enter.
expected: docs/tui.md "In the Terminal window every key goes to the shell except the window and Find Action keys above"; IntelliJ shows "Process finished" and restarts on Enter.
actual: (a) chord swallowed. (b) the window keeps showing `exit`, no message, further typing does nothing, a toast "command failed: terminal input failed" appears when typing; recovery only via Find Action "Restart Terminal".
terminal mode: legacy

### TUI-BUG-023 Mouse: no context menus; Ln/Col segment click does nothing
severity: minor
area: mouse
steps: right-click in the editor or on a tree item; click the "Ln 8, Col 3" status segment.
expected: IntelliJ context menu; Ln/Col click opens Go to Line. docs/tui.md "the mouse works everywhere (... status line segments)".
actual: right-click only moves the caret / selects the row; Ln/Col click ignored (the run segment, project and problem counters do react).
terminal mode: xterm

### TUI-BUG-025 Which-key/chord ESC-prefix, Alt+f/b and rxvt-style sequences are inserted as text or ignored
severity: minor
area: keys / legacy terminals
steps: send ESC ESC [ C (Alt+Right on urxvt/Linux console), ESC f / ESC b (Option+Right/Left on macOS Terminal).
expected: next tab / word movement (macOS Option+arrows are the common way).
actual: `ESC ESC [ C` inserts the text "[C" into the document; ESC f / ESC b do nothing; ESC [5D and ESC O d (rxvt Ctrl+Left) jump to the line start. Ctrl+H (^H, sent by Backspace on many terminals/ssh clients) deletes a whole word (documented fallback, but Backspace-as-^H users lose characters).
terminal mode: legacy

### TUI-BUG-028 Misc minor feedback issues
severity: minor
area: status line / Find in Files / dialogs
steps/actual: (a) Find in Files "lorem": window header "500 results in 1 file" but status "Found 500 results in 7 files (limit reached)" (also "2 results in 2 files" vs "Found 2 results in 3 files"). (b) `archipel-ide --tui --help` treats --help as a directory; `--tui <file>` as project gives "project path is not a directory". (c) bad file argument message "filesystem error: No …" cut off. (d) project tree speed search drops `=` and `(` when typing. (e) completion popup is clipped at the right screen edge instead of shifting left (100 cols: "SystemSleepListen", "System.out.printl"). (f) status messages stay stale ("Opened pom.xml", "Indexing 0 / 0 files", "Project imported" alternating). (g) Alt+Enter at a line shows tiny "Generate Getters" etc. though Generate is "not available".
terminal mode: legacy

### TUI-BUG-029 Enter inside a Java string literal does not split it
severity: minor
area: editor typing
steps: App.java: `String s = "x` (auto-pair gives "x") Enter.
expected: spec ED-TYPE-068 (SHOULD): `"x" +` newline `"…"`.
actual: literal broken over two lines (`String s = "x` / `"`), syntax error.
terminal mode: legacy

### TUI-BUG-006 Splash logo is misspelled
severity: cosmetic
area: dashboard
steps: start `archipel-ide --tui` (no file).
expected: the product name ARCHIPEL.
actual: block letters read "ARCHIFL" / "ARCHIEL" (7 glyphs: A R C H I E/F L, the P and E are missing/garbled):
  ▄▀█ █▀█ █▀▀ █ █ █ █▀▀ █
  █▀█ █▀▄ █▄▄ █▀█ █ █▀▀ █▄▄
terminal mode: both

### TUI-BUG-024 Light theme leaves a dark patch in the tab line; status bar shows git icon in non-git project
severity: cosmetic
area: rendering / theming
steps: txt project (no git), click the sun/theme button at the top right of the tab line once (light theme).
expected: tab line uses theme background everywhere; no VCS icon without a repository.
actual: a dark rectangle remains right of the last tab (x 1025-1075 in the screenshot) in the light theme; in a non-git project the status line shows an orange branch glyph with no text after the file name.
terminal mode: xterm


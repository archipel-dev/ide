# Find in Files navigation, reveal in tree, linked directories (#47 bugs 10, 11, 13)

Captured 2026-10-10 on Linux/Xvfb (glow) with `xdotool`. The sample project has
`app/src/main/java/com/example/Greeter.java` (line 9 is
`String greeting = greet(greeting("x"));`) and `bazel-bin`, a symbolic link to a
directory outside the project, like Bazel's output links.

## 11. Find in Files

Before: one row per occurrence (a line with two hits was listed twice), no highlight,
and clicking opened the file at 1:1. The search also sliced the original line with
byte offsets taken from its lowercase copy, which could panic on characters whose
lowercase form has a different length (`İ`).

After: one row per line with every occurrence highlighted (`TEXT_SEARCH_RESULT`
background). Clicking opens the file with the first occurrence selected
(`editor.openFile {path, line, column, length}`) and moves focus to the editor.

![One row per line, three highlighted occurrences on line 9](workbench-bugs/find-in-files-highlights.png)

Tests: `a_line_with_two_hits_is_one_row_with_two_highlights`,
`case_folding_that_changes_byte_length_does_not_panic` (archipel-ide-search),
`find_preview_highlights_each_occurrence` (UI),
`open_file_lands_on_and_selects_a_search_match` (core).

## 13. Reveal the opened file in the Project tree

Opening a file from any source (CLI argument, Find in Files, Go to File, Recent Files,
tabs) expands its parent directories, selects it in the tree, and scrolls it into view
when it is off-screen. This matches the reference IDE with "Always Select Opened File"
turned on.

![Opened at 9:16 with "greet" selected; tree expanded to Greeter.java](workbench-bugs/find-open-at-match-reveal.png)

Tests: `opening_a_nested_file_reveals_it_in_the_tree` (core),
`reveal_scrolls_only_when_the_row_is_hidden` (UI).

## 10. Directory symlinks

Before: `bazel-bin` and similar links were drawn as files, and opening one failed with
"path is outside the project".

After: a link to a directory is a folder with a chevron. Its contents are listed when it
is expanded, not walked at startup, because output trees can be huge. Files inside it
open for reading. Writing, creating and renaming through a link that points outside the
project are still refused, and deleting the link removes only the link.

![bazel-bin expanded lazily, generated file opened](workbench-bugs/linked-directory-expanded.png)

Tests: `linked_directories_are_listed_lazily_and_readable_but_not_writable` (project),
`linked_directories_expand_and_their_files_open` (core).

## Gaps

- No Match case / Words / Regex toggles, file masks, scopes or preview pane in Find in Files yet.
- Linked output directories are not shown as excluded (dimmed) yet.
- The "Always Select Opened File" behaviour can't be switched off yet.
